use criterion::*;
use wad::map::Map;
use wad::wad::Wad;

fn bench_find_e1m1(b: &mut Bencher, _i: &u32) {
    let mut wad = Wad::new("../doom1.wad");
    wad.read_directories();
    b.iter(|| {
        let _index = wad.find_lump_index("E1M1");
    });
}

fn bench_load_e1m1(b: &mut Bencher, _i: &u32) {
    let mut wad = Wad::new("../doom1.wad");
    wad.read_directories();
    let mut map = Map::new("E1M1".to_owned());
    b.iter(|| {
        wad.load_map(&mut map);
    });
}

fn bench_load_e1m7(b: &mut Bencher, _i: &u32) {
    let mut wad = Wad::new("../doom1.wad");
    wad.read_directories();
    let mut map = Map::new("E1M7".to_owned());
    b.iter(|| {
        wad.load_map(&mut map);
    });
}

fn bench(c: &mut Criterion) {
    let fun = vec![
        Fun::new("Load e1m1 from shareware", bench_load_e1m1),
        Fun::new("Load e1m7 from shareware", bench_load_e1m7),
    ];
    c.bench_functions("Loading E1M1", fun, 10);
}

criterion_group!(benches, bench,);
criterion_main!(benches);