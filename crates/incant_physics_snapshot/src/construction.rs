//! Default-off shared precharged construction payloads, with no solver dependency.
//! Logical requests exclude allocator overhead; Box/Arc or downstream VecMap
//! allocation may still abort on process OOM. This utility constructs no owner.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstructionError {
    Limit,
    Allocation,
    Generation,
    Payload,
    Incomplete,
}
pub type ConstructionResult<T> = Result<T, ConstructionError>;
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct AllocationPlan {
    bytes: usize,
    allocations: usize,
}
impl AllocationPlan {
    pub fn array<T>(count: usize) -> ConstructionResult<Self> {
        let bytes = count
            .checked_mul(size_of::<T>())
            .filter(|n| *n <= isize::MAX as usize)
            .ok_or(ConstructionError::Limit)?;
        Ok(Self {
            bytes,
            allocations: usize::from(bytes != 0),
        })
    }
    pub fn append(&mut self, other: Self) -> ConstructionResult<()> {
        let bytes = self
            .bytes
            .checked_add(other.bytes)
            .ok_or(ConstructionError::Limit)?;
        let allocations = self
            .allocations
            .checked_add(other.allocations)
            .ok_or(ConstructionError::Limit)?;
        *self = Self { bytes, allocations };
        Ok(())
    }
    pub fn bytes(&self) -> usize {
        self.bytes
    }
    pub fn allocations(&self) -> usize {
        self.allocations
    }
}
pub struct AllocationBudget {
    bytes: usize,
    allocations: usize,
}
impl AllocationBudget {
    pub fn new(bytes: usize, allocations: usize) -> Self {
        Self { bytes, allocations }
    }
    pub fn admit(&mut self, plan: AllocationPlan) -> ConstructionResult<AllocationPermit> {
        let bytes = self
            .bytes
            .checked_sub(plan.bytes)
            .ok_or(ConstructionError::Limit)?;
        let allocations = self
            .allocations
            .checked_sub(plan.allocations)
            .ok_or(ConstructionError::Limit)?;
        *self = Self { bytes, allocations };
        Ok(AllocationPermit { remaining: plan })
    }
}
/// Non-Clone permit. Failed later allocations or payload conversion do not refund
/// the outer shared budget; no partially constructed owner is published.
pub struct AllocationPermit {
    remaining: AllocationPlan,
}
impl AllocationPermit {
    pub fn charge(&mut self, need: AllocationPlan) -> ConstructionResult<()> {
        let bytes = self
            .remaining
            .bytes
            .checked_sub(need.bytes)
            .ok_or(ConstructionError::Limit)?;
        let allocations = self
            .remaining
            .allocations
            .checked_sub(need.allocations)
            .ok_or(ConstructionError::Limit)?;
        self.remaining = AllocationPlan { bytes, allocations };
        Ok(())
    }
    /// Stable Box has no fallible allocation API. The scalar payload is charged
    /// before allocation; process allocator exhaustion retains Rust's ordinary
    /// abort behavior. This does not claim recoverable OOM for Box/Arc owners.
    pub fn boxed<T>(&mut self, value: T) -> ConstructionResult<Box<T>> {
        self.charge(AllocationPlan::array::<T>(1)?)?;
        Ok(Box::new(value))
    }
    pub fn vector<T>(&mut self, count: usize) -> ConstructionResult<Vec<T>> {
        self.charge(AllocationPlan::array::<T>(count)?)?;
        let mut v = Vec::new();
        v.try_reserve_exact(count)
            .map_err(|_| ConstructionError::Allocation)?;
        // Arena::clear observes its Vec capacity. Do not silently construct a
        // different free-slot extent if a platform returns a larger capacity.
        if size_of::<T>() != 0 && v.capacity() != count {
            return Err(ConstructionError::Allocation);
        }
        Ok(v)
    }
    pub fn finish(self) -> ConstructionResult<()> {
        if self.remaining == AllocationPlan::default() {
            Ok(())
        } else {
            Err(ConstructionError::Incomplete)
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checked_storage_allocation_plan_is_cumulative_and_precharged() {
        let a = AllocationPlan::array::<u64>(3).unwrap();
        let mut plan = a;
        plan.append(AllocationPlan::array::<u32>(5).unwrap())
            .unwrap();
        assert_eq!((plan.bytes(), plan.allocations()), (44, 2));
        let mut small = AllocationBudget::new(43, 2);
        assert!(matches!(small.admit(plan), Err(ConstructionError::Limit)));
        assert_eq!((small.bytes, small.allocations), (43, 2));
        let mut few = AllocationBudget::new(44, 1);
        assert!(matches!(few.admit(plan), Err(ConstructionError::Limit)));
        let mut budget = AllocationBudget::new(88, 4);
        let mut permit = budget.admit(plan).unwrap();
        assert_eq!((budget.bytes, budget.allocations), (44, 2));
        let _: Vec<u64> = permit.vector(3).unwrap();
        let _: Vec<u32> = permit.vector(5).unwrap();
        assert!(permit.vector::<u8>(1).is_err());
        permit.finish().unwrap();
        {
            let _unused = budget.admit(plan).unwrap();
        }
        assert!(budget.admit(a).is_err());
        assert!(AllocationPlan::array::<u64>(usize::MAX).is_err());
        assert!(
            AllocationBudget::new(24, 1)
                .admit(a)
                .unwrap()
                .finish()
                .is_err()
        );
    }
}
