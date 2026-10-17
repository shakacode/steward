use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use std::collections::HashMap;
use steward::{env::PATH, Env};

fn env_construction(c: &mut Criterion) {
    let mut group = c.benchmark_group("env_construction");

    group.bench_function("empty", |b| b.iter(Env::empty));

    group.bench_function("one", |b| {
        b.iter(|| Env::one(black_box("KEY"), black_box("VALUE")))
    });

    group.bench_function("from_vec_5", |b| {
        b.iter(|| {
            Env::from_vec(vec![
                ("A", "1"),
                ("B", "2"),
                ("C", "3"),
                ("D", "4"),
                ("E", "5"),
            ])
        })
    });

    group.bench_function("from_vec_20", |b| {
        let pairs: Vec<(String, String)> = (0..20)
            .map(|i| (format!("KEY_{i}"), format!("VAL_{i}")))
            .collect();
        b.iter(|| {
            Env::from_vec(
                pairs
                    .iter()
                    .map(|(k, v)| (k.as_str(), v.as_str()))
                    .collect::<Vec<_>>(),
            )
        })
    });

    group.bench_function("from_hashmap_20", |b| {
        let mut map = HashMap::with_capacity(20);
        for i in 0..20 {
            map.insert(format!("KEY_{i}"), format!("VAL_{i}"));
        }
        b.iter(|| Env::new(black_box(map.clone())))
    });

    group.finish();
}

fn env_clone_vs_ref(c: &mut Criterion) {
    let mut group = c.benchmark_group("env_clone_vs_ref");

    for size in [5, 10, 20, 50] {
        let env = Env::from_vec(
            (0..size)
                .map(|i| (format!("KEY_{i}"), format!("VAL_{i}")))
                .collect::<Vec<_>>(),
        );

        group.bench_with_input(BenchmarkId::new("clone", size), &size, |b, _| {
            b.iter(|| {
                let _cloned = black_box(&env).clone();
            })
        });

        group.bench_with_input(BenchmarkId::new("iter_ref", size), &size, |b, _| {
            b.iter(|| {
                let mut count = 0;
                for (_k, _v) in &env {
                    count += 1;
                }
                black_box(count);
            })
        });

        group.bench_with_input(BenchmarkId::new("iter_owned", size), &size, |b, _| {
            b.iter(|| {
                let env_clone = env.clone();
                let mut count = 0;
                for (_k, _v) in env_clone {
                    count += 1;
                }
                black_box(count);
            })
        });
    }

    group.finish();
}

fn env_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("env_operations");

    group.bench_function("insert_chain_5", |b| {
        b.iter(|| {
            Env::empty()
                .insert(black_box("A"), black_box("1"))
                .insert(black_box("B"), black_box("2"))
                .insert(black_box("C"), black_box("3"))
                .insert(black_box("D"), black_box("4"))
                .insert(black_box("E"), black_box("5"))
        })
    });

    group.bench_function("insert_cloned_5", |b| {
        let base = Env::from_vec(vec![("X", "0")]);
        b.iter(|| {
            let mut env = base.clone();
            for i in 0..5 {
                env = env.insert_cloned(black_box(format!("K_{i}")), black_box(format!("V_{i}")));
            }
            black_box(env);
        })
    });

    group.bench_function("extend_10_plus_10", |b| {
        let a = Env::from_vec((0..10).map(|i| (format!("A_{i}"), "1")).collect::<Vec<_>>());
        let b_env = Env::from_vec((0..10).map(|i| (format!("B_{i}"), "2")).collect::<Vec<_>>());
        b.iter(|| {
            let _merged = black_box(&a).clone().extend(black_box(&b_env).clone());
        })
    });

    group.bench_function("extend_cloned_10_plus_10", |b| {
        let a = Env::from_vec((0..10).map(|i| (format!("A_{i}"), "1")).collect::<Vec<_>>());
        let b_env = Env::from_vec((0..10).map(|i| (format!("B_{i}"), "2")).collect::<Vec<_>>());
        b.iter(|| {
            let _merged = a.extend_cloned(black_box(b_env.clone()));
        })
    });

    group.finish();
}

fn path_operations(c: &mut Criterion) {
    c.bench_function("path_extend", |b| {
        b.iter(|| PATH::extend(black_box("/custom/bin")))
    });

    c.bench_function("path_get", |b| b.iter(PATH::get));
}

criterion_group!(
    benches,
    env_construction,
    env_clone_vs_ref,
    env_operations,
    path_operations,
);
criterion_main!(benches);
