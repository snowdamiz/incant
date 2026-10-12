//! Public package acceptance guest; numeric columns carry physics observations.
#![no_std]
use incant_module::{EntityId, Error, IDENTITY, LogLevel, Result, Tick, physics::*};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
incant_module::module!(simulate);
static mut INPUT: [u8; incant_module::INPUT_MAX_BYTES] = [0; incant_module::INPUT_MAX_BYTES];
fn queries() -> [Query; 7] {
    let ray = Ray3 {
        origin: [-8., 1., 0.],
        direction: [2., 0., 0.],
        max_distance: 20.,
        include_sensors: false,
        exclude_entity: Some(EntityId::from_u128(11)),
        memberships: u32::MAX,
        filter: u32::MAX,
    };
    let planar = Ray2 {
        origin: [0.5, 3.],
        direction: [0., -2.],
        max_distance: 10.,
        include_sensors: false,
        exclude_entity: Some(EntityId::from_u128(2)),
        memberships: u32::MAX,
        filter: u32::MAX,
    };
    [
        Query::Ray3(ray),
        Query::Ray3(Ray3 {
            include_sensors: true,
            ..ray
        }),
        Query::Ray3(Ray3 { filter: 0, ..ray }),
        Query::Ray2(planar),
        Query::Ray2(Ray2 {
            include_sensors: true,
            ..planar
        }),
        Query::Ray2(Ray2 {
            origin: [1.5, 3.],
            ..planar
        }),
        Query::Character(Character {
            entity: EntityId::from_u128(11),
            translation: [5., -1., 0.],
            options: CharacterOptions::default(),
        }),
    ]
}
fn hit<const N: usize>(h: Option<RayHit<N>>) -> f64 {
    let Some(h) = h else { return -1. };
    let mut n = h.distance + f64::from(h.entity.0[15]);
    for (i, (&p, &v)) in h.point.iter().zip(&h.normal).enumerate() {
        n += p * (i + 1) as f64 + v * (i + 5) as f64;
    }
    n
}
fn score(r: Results<'_>) -> (f64, [f64; 3]) {
    let mut sum = 0.;
    let mut motion = [0.; 3];
    for (i, r) in r.iter().enumerate() {
        sum += (i + 1) as f64
            * match r {
                QueryResult::Ray3(h) => hit(h),
                QueryResult::Ray2(h) => hit(h),
                QueryResult::Character(c) => {
                    motion = c.translation;
                    let mut n = f64::from(u8::from(c.grounded))
                        + 2. * f64::from(u8::from(c.sliding_down_slope));
                    for (i, v) in c.translation.into_iter().enumerate() {
                        n += v * (i + 1) as f64;
                    }
                    for id in c.collisions() {
                        n += f64::from(id.0[15]);
                    }
                    n
                }
            };
    }
    (sum, motion)
}
fn events(e: Events<'_>) -> f64 {
    let mut sum = 0.;
    for e in e.iter() {
        sum += f64::from(e.first.0[15])
            + f64::from(e.second.0[15]) * 7.
            + f64::from(u8::from(e.entered)) * 101.;
    }
    sum
}
fn simulate(tick: &mut Tick<'_>, _: f64) -> Result<()> {
    // Synchronous host entry; the scratch view never escapes this callback.
    let input = tick.input(unsafe { &mut *(&raw mut INPUT) })?;
    let clock = input.tick;
    let failed = (1..=4).find(|i| {
        input
            .action(match i {
                1 => "bad_owner",
                2 => "short_scratch",
                3 => "bad_number",
                _ => "late_command",
            })
            .is_some_and(|a| a.active)
    });
    let q = queries();
    let mut request = [0; 1024];
    let mut result = [0; 4096];
    let rows = tick.query_physics(&q, &mut request, &mut result)?;
    let (checksum, motion) = score(rows);
    let mut planar = [0; 8192];
    let mut spatial = [0; 8192];
    let e2 = tick.physics_events(Dimension::Planar, &mut planar)?;
    let e3 = tick.physics_events(Dimension::Spatial, &mut spatial)?;
    if e2.scene() != EntityId::from_u128(99)
        || e3.scene() != e2.scene()
        || e2.dimension() != Dimension::Planar
        || e3.dimension() != Dimension::Spatial
    {
        return Err(Error::Malformed);
    }
    let event_score = events(e2) + events(e3) * 1000.;
    let mut repeat = [0; 8192];
    if events(tick.physics_events(Dimension::Spatial, &mut repeat)?) != events(e3) {
        return Err(Error::Malformed);
    }
    for index in 0..tick.chunk_count()? {
        let chunk = tick.chunk(index)?;
        let mut ids = [EntityId([0; 16]); 16];
        let ids = tick.read_ids(chunk, &mut ids)?;
        let mut ts = [IDENTITY; 16];
        let ts = tick.read_transforms(chunk, &mut ts)?;
        for (id, t) in ids.iter().zip(ts.iter_mut()) {
            if *id == EntityId::from_u128(20) {
                t[0] = checksum;
                t[1] = event_score;
                t[2] += 1.;
            }
            if *id == EntityId::from_u128(11) {
                for i in 0..3 {
                    t[i] += motion[i];
                }
                if clock == 3 {
                    t[2] += 10.;
                }
            }
            if *id == EntityId::from_u128(2) {
                t[0] += 10.;
            }
        }
        tick.write_transforms(chunk, ts)?;
    }
    let mut after = [0; 4096];
    let again = tick.query_physics(&q, &mut request, &mut after)?;
    if score(again) != score(rows) || after != result {
        return Err(Error::Malformed);
    }
    if e2.scene() != EntityId::from_u128(99) || e3.scene() != e2.scene() {
        return Err(Error::Malformed);
    }
    tick.log("physics queries accepted", LogLevel::Info)?;
    if let Some(failed) = failed {
        tick.spawn(&EntityId::from_u128(77), None, &IDENTITY, None)?;
        tick.log("failure prefix must not publish", LogLevel::Warn)?;
        match failed {
            1 => {
                let _ = tick.query_physics(
                    &[Query::Character(Character {
                        entity: EntityId::from_u128(12),
                        translation: [0.1, 0., 0.],
                        options: CharacterOptions::default(),
                    })],
                    &mut request,
                    &mut result,
                );
            }
            2 => {
                let _ = tick.query_physics(&q, &mut request, &mut []);
            }
            3 => {
                let _ = tick.query_physics(
                    &[Query::Ray3(Ray3 {
                        origin: [1_000_000.000_001, 0., 0.],
                        direction: [1., 0., 0.],
                        max_distance: 1.,
                        include_sensors: false,
                        exclude_entity: None,
                        memberships: u32::MAX,
                        filter: u32::MAX,
                    })],
                    &mut request,
                    &mut result,
                );
            }
            _ => tick.set_velocity(&EntityId::from_u128(123), Some(&[1., 0., 0.]))?,
        }
    }
    Ok(())
}
