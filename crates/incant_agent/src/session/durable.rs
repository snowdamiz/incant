//! Filesystem accounting runs on the accepted worker, outside controller locks.
#[cfg(test)]
mod tests;
use super::*;
use crate::ledger::MAX_TOKEN_CAP;

pub(super) fn validate_snapshot(snapshot: &LedgerSnapshot) -> Result<(), SessionError> {
    if snapshot.budget.max_tokens > MAX_TOKEN_CAP || snapshot.budget.used_tokens > MAX_TOKEN_CAP {
        return Err(SessionError::InvalidRequest);
    }
    Ok(())
}

pub(super) fn run(
    context: TurnContext,
    ledger: Ledger,
    job: impl FnOnce(TurnContext) -> TurnOutcome,
) -> Completion {
    let initial = context.budget.clone();
    let cancelled = |context: &TurnContext| context.cancel.load(Ordering::Acquire);
    let interrupted = || TurnOutcome {
        budget: initial.clone(),
        result: Err(AgentError::Interrupted),
    };
    if cancelled(&context) {
        return prepare(&initial, Some(interrupted()));
    }
    let lease = match ledger.reserve(initial.max_tokens) {
        Ok(lease) => lease,
        Err(_) => return conservative(&initial, Failure::BudgetAccounting),
    };
    // A cancellation racing reservation still invokes no provider and has an
    // exact zero-work receipt. Never refund an interrupted provider by inference.
    let outcome = if cancelled(&context) {
        Some(interrupted())
    } else {
        catch_unwind(AssertUnwindSafe(|| job(context))).ok()
    };
    let mut completion = prepare(&initial, outcome);
    if !completion.valid_receipt {
        return completion; // Dropping the lease retains the full reservation.
    }
    // Historical ledger uncertainty already lives in the restored session;
    // do not label a new measured turn uncertain merely because an earlier one
    // crashed. Numeric saturation, however, affects this turn's own receipt.
    completion.uncertain |= completion.budget.used_tokens > MAX_TOKEN_CAP;
    match lease.complete(&completion.budget, completion.uncertain) {
        Ok(snapshot) => {
            completion.budget = snapshot.budget;
            completion
        }
        Err(_) => {
            // Retain every validated measured token we know in memory, even
            // beyond the reserved cap. Unavailable storage may still hold only
            // the original reservation, never an exact later overrun receipt.
            let mut failed = conservative(&initial, Failure::BudgetAccounting);
            failed.budget.used_tokens = failed
                .budget
                .used_tokens
                .max(completion.budget.used_tokens.min(MAX_TOKEN_CAP));
            failed
        }
    }
}
