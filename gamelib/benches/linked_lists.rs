use std::ptr::NonNull;

use criterion::*;

use gamelib::{
    d_main,
    d_thinker::{ActionF, TestObject, Think, ThinkerAlloc, ThinkerType},
    doom_def,
    level_data::{level::Level, map_data::MapData},
};
use wad::WadData;

fn push_100_000(b: &mut Bencher) {
    let mut links = unsafe { ThinkerAlloc::new(10000) };
    b.iter(|| {
        for i in 0..10000 {
            links.push::<TestObject>(TestObject::create_thinker(
                ThinkerType::Test(TestObject {
                    x: i,
                    thinker: NonNull::dangling(),
                }),
                ActionF::None,
            ));
        }
    });
}

fn load_and_iter(b: &mut Bencher) {
    let wad = WadData::new("../doom1.wad".into());
    let mut map = MapData::new("E1M1".to_owned());
    map.load(&wad);

    let mut level = unsafe { Level::new(d_main::Skill::Baby, 1, 1, doom_def::GameMode::Shareware) };

    let mut links = unsafe { ThinkerAlloc::new(10000) };

    for i in 0..10000 {
        links.push::<TestObject>(TestObject::create_thinker(
            ThinkerType::Test(TestObject {
                x: i,
                thinker: NonNull::dangling(),
            }),
            ActionF::None,
        ));
    }

    b.iter(|| {
        let mut _count = 0;
        links.run_thinkers(&mut level);
    });
}

fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("ThinkerAlloc stressing");

    // 10,000 seems to be the breaking point between fast and terribly slow
    group.bench_function("Push 10,000", push_100_000);
    group.bench_function("Iterate over 10,000", load_and_iter);
}

criterion_group!(benches, bench,);
criterion_main!(benches);
