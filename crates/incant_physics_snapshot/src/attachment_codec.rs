use crate::{
    AttachmentRecord, AttachmentSpan, Budget, Error, Handle, LocalPose, MAX_BYTES,
    ParentAttachment, Result, ValidatedArena, ValidatedAttachments,
    arena_codec::{read_identity, write_identity},
    attachment::attachment_lengths,
    codec::Reader,
    validate_attachments, vector,
};
const MAGIC: &[u8; 8] = b"ICATT001";
const HEADER: usize = 60;

fn length(b: usize, c: usize, a: usize) -> Result<usize> {
    b.checked_mul(8)
        .and_then(|v| c.checked_mul(40).and_then(|c| v.checked_add(c)))
        .and_then(|v| a.checked_mul(8).and_then(|a| v.checked_add(a)))
        .and_then(|v| v.checked_add(HEADER))
        .filter(|v| *v <= MAX_BYTES)
        .ok_or(Error::Limit)
}

pub fn decode_attachments<'a>(
    bytes: &[u8],
    bodies: &'a ValidatedArena,
    colliders: &'a ValidatedArena,
    budget: &mut Budget,
) -> Result<ValidatedAttachments<'a>> {
    if !(HEADER..=MAX_BYTES).contains(&bytes.len()) {
        return Err(Error::Length);
    }
    let identity = bodies.record().identity;
    let mut read = Reader { bytes, position: 0 };
    read_identity(&mut read, MAGIC, identity)?;
    let b = read.word()? as usize;
    let c = read.word()? as usize;
    let a = read.word()? as usize;
    if read.word()? != 0 {
        return Err(Error::Header);
    }
    attachment_lengths(identity, [b, c, a], bodies, colliders)?;
    if bytes.len() != length(b, c, a)? {
        return Err(Error::Length);
    }
    budget.charge(
        b.checked_mul(size_of::<AttachmentSpan>())
            .and_then(|v| {
                c.checked_mul(size_of::<Option<ParentAttachment>>())
                    .and_then(|c| v.checked_add(c))
            })
            .and_then(|v| {
                a.checked_mul(size_of::<Handle>())
                    .and_then(|a| v.checked_add(a))
            })
            .ok_or(Error::Limit)?,
    )?;
    let mut record = AttachmentRecord {
        identity,
        bodies: vector(b)?,
        colliders: vector(c)?,
        attached: vector(a)?,
    };
    for _ in 0..b {
        record.bodies.push(AttachmentSpan {
            start: read.word()?,
            count: read.word()?,
        });
    }
    for _ in 0..c {
        let mut words = [0u32; 10];
        for word in &mut words {
            *word = read.word()?;
        }
        record.colliders.push(match words[0] {
            0 if words[1..].iter().all(|v| *v == 0) => None,
            1 => Some(ParentAttachment {
                body: Handle {
                    slot: words[1],
                    generation: words[2],
                },
                local_pose: LocalPose {
                    translation: [words[3], words[4], words[5]],
                    rotation: [words[6], words[7], words[8], words[9]],
                },
            }),
            _ => return Err(Error::Header),
        });
    }
    for _ in 0..a {
        record.attached.push(Handle {
            slot: read.word()?,
            generation: read.word()?,
        });
    }
    validate_attachments(record, bodies, colliders, budget)
}

pub fn encode_attachments(
    value: &ValidatedAttachments<'_>,
    budget: &mut Budget,
) -> Result<Vec<u8>> {
    let record = value.record();
    let len = length(
        record.bodies.len(),
        record.colliders.len(),
        record.attached.len(),
    )?;
    budget.charge(len)?;
    let mut bytes = vector(len)?;
    write_identity(&mut bytes, MAGIC, record.identity);
    for word in [
        record.bodies.len() as u32,
        record.colliders.len() as u32,
        record.attached.len() as u32,
        0,
    ] {
        bytes.extend_from_slice(&word.to_le_bytes());
    }
    for span in &record.bodies {
        bytes.extend_from_slice(&span.start.to_le_bytes());
        bytes.extend_from_slice(&span.count.to_le_bytes());
    }
    for parent in &record.colliders {
        let words = parent.map_or([0; 10], |p| {
            [
                1,
                p.body.slot,
                p.body.generation,
                p.local_pose.translation[0],
                p.local_pose.translation[1],
                p.local_pose.translation[2],
                p.local_pose.rotation[0],
                p.local_pose.rotation[1],
                p.local_pose.rotation[2],
                p.local_pose.rotation[3],
            ]
        });
        for word in words {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
    }
    for handle in &record.attached {
        bytes.extend_from_slice(&handle.slot.to_le_bytes());
        bytes.extend_from_slice(&handle.generation.to_le_bytes());
    }
    Ok(bytes)
}
