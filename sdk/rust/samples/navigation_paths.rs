//! Copied committed routes drive an ordinary entity and observe the next rebuild.
#![no_std]
use incant_module::{navigation::*, EntityId, Error, IDENTITY, LogLevel, Result, Tick};
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>)->! { core::arch::wasm32::unreachable() }
incant_module::module!(simulate);
fn score(path:Option<Path<'_>>) -> (f64,f64) {
    let Some(p)=path else{return (0.,0.)};
    let mut sum=p.visited as f64;
    for (i,point) in p.points().enumerate(){for (axis,n) in point.into_iter().enumerate(){sum+=n*(i*3+axis+1) as f64;}}
    for n in p.corridor(){sum+=n as f64;}
    for t in p.traversals(){sum+=(t.from_index+t.to_index*17+t.reversed*31+u32::from(t.link_id[15])) as f64;}
    (sum,p.generation as f64)
}
fn simulate(tick:&mut Tick<'_>,_:f64)->Result<()> {
    let owner=EntityId::from_u128(1);
    let q=PathRequest{start:[-8.,0.,0.],end:[8.,0.,0.],snap_distance:1.,max_visited:1000};
    let mut mesh=[0;8192];let mut grid=[0;1024];let mut reverse=[0;8192];
    let p=tick.find_path(&owner,q,&mut mesh)?;
    let g=tick.find_grid_path(&owner,GridPathRequest{start:[0,0],end:[2,0],diagonal:true,max_expansions:100},&mut grid)?.ok_or(Error::Malformed)?;
    let reversed=tick.find_path(&owner,PathRequest{start:q.end,end:q.start,..q},&mut reverse)?;
    let (mut checksum,generation)=score(p);checksum+=score(reversed).0+g.cost as f64;
    for (i,c) in g.cells().enumerate(){checksum+=(i as f64+1.)*(c[0] as f64+c[1] as f64*7.);}
    for i in 0..tick.chunk_count()? {
        let chunk=tick.chunk(i)?;let mut ids=[EntityId([0;16]);16];let ids=tick.read_ids(chunk,&mut ids)?;
        let mut transforms=[IDENTITY;16];let rows=tick.read_transforms(chunk,&mut transforms)?;
        for (id,t) in ids.iter().zip(rows.iter_mut()) {
            if *id==EntityId::from_u128(4){t[0]=checksum;t[1]=generation+g.generation as f64;t[2]+=1.;}
            if *id==EntityId::from_u128(3){t[2]=4.;}
        }
        tick.write_transforms(chunk,rows)?;
    }
    // Pending source movement is invisible to this callback's cache, and a
    // subsequent reply must not invalidate the caller-owned earlier result.
    let mut after=[0;8192];let same=tick.find_path(&owner,q,&mut after)?;
    if score(same)!=score(p) || after!=mesh {return Err(Error::Malformed);}
    let mut miss=[0;64];
    if tick.find_path(&owner,PathRequest{start:[200.,0.,0.],..q},&mut miss)?.is_some(){return Err(Error::Malformed);}
    if tick.find_grid_path(&owner,GridPathRequest{start:[0,0],end:[1,0],diagonal:true,max_expansions:100},&mut miss)?.is_some(){return Err(Error::Malformed);}
    tick.log("navigation path accepted",LogLevel::Info)?;
    Ok(())
}
