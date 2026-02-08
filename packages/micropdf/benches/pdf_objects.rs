use criterion::{Criterion, black_box, criterion_group, criterion_main};
use micropdf::pdf::object::{Name, ObjRef, Object, PdfString};
use std::collections::HashMap;
use std::f64::consts::PI;

fn bench_object_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("pdf/object/create");

    group.bench_function("null", |b| b.iter(|| Object::Null));

    group.bench_function("bool", |b| b.iter(|| Object::Bool(black_box(true))));

    group.bench_function("int", |b| b.iter(|| Object::Int(black_box(42))));

    group.bench_function("real", |b| b.iter(|| Object::Real(black_box(PI))));

    group.bench_function("name", |b| {
        b.iter(|| Object::Name(Name::new(black_box("Type"))))
    });

    group.bench_function("string", |b| {
        b.iter(|| Object::String(PdfString::new(black_box(b"Hello, World!".to_vec()))))
    });

    group.bench_function("ref", |b| {
        b.iter(|| Object::Ref(ObjRef::new(black_box(1), black_box(0))))
    });

    group.finish();
}

fn bench_name_operations(c: &mut Criterion) {
    let name1 = Name::new("Type");
    let name2 = Name::new("Type");
    let name3 = Name::new("Subtype");

    let mut group = c.benchmark_group("pdf/name");

    group.bench_function("new", |b| b.iter(|| Name::new(black_box("FontDescriptor"))));

    group.bench_function("eq_same", |b| {
        b.iter(|| black_box(&name1) == black_box(&name2))
    });

    group.bench_function("eq_diff", |b| {
        b.iter(|| black_box(&name1) == black_box(&name3))
    });

    group.bench_function("to_string", |b| b.iter(|| black_box(&name1).to_string()));

    group.finish();
}

fn bench_string_operations(c: &mut Criterion) {
    let literal = PdfString::new(b"Hello, World!".to_vec());

    let mut group = c.benchmark_group("pdf/string");

    group.bench_function("new", |b| {
        b.iter(|| PdfString::new(black_box(b"Test string content".to_vec())))
    });

    group.bench_function("as_bytes", |b| b.iter(|| black_box(&literal).as_bytes()));

    group.finish();
}

fn bench_array_operations(c: &mut Criterion) {
    let small_array: Vec<Object> = (0..10).map(Object::Int).collect();
    let large_array: Vec<Object> = (0..1000).map(Object::Int).collect();

    let mut group = c.benchmark_group("pdf/array");

    group.bench_function("create_10", |b| {
        b.iter(|| Object::Array((0..10).map(|i| Object::Int(black_box(i))).collect()))
    });

    group.bench_function("create_1000", |b| {
        b.iter(|| Object::Array((0..1000).map(|i| Object::Int(black_box(i))).collect()))
    });

    group.bench_function("access_10", |b| {
        let arr = Object::Array(small_array.clone());
        b.iter(|| {
            if let Object::Array(ref a) = arr {
                black_box(&a[5])
            } else {
                panic!()
            }
        })
    });

    group.bench_function("access_1000", |b| {
        let arr = Object::Array(large_array.clone());
        b.iter(|| {
            if let Object::Array(ref a) = arr {
                black_box(&a[500])
            } else {
                panic!()
            }
        })
    });

    group.finish();
}

fn bench_dict_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("pdf/dict");

    // Small dictionary (typical PDF object)
    let mut small_dict: HashMap<Name, Object> = HashMap::new();
    small_dict.insert(Name::new("Type"), Object::Name(Name::new("Page")));
    small_dict.insert(Name::new("Parent"), Object::Ref(ObjRef::new(1, 0)));
    small_dict.insert(
        Name::new("MediaBox"),
        Object::Array(vec![
            Object::Int(0),
            Object::Int(0),
            Object::Int(612),
            Object::Int(792),
        ]),
    );

    group.bench_function("create_small", |b| {
        b.iter(|| {
            let mut dict: HashMap<Name, Object> = HashMap::new();
            dict.insert(Name::new("Type"), Object::Name(Name::new("Page")));
            dict.insert(Name::new("Parent"), Object::Ref(ObjRef::new(1, 0)));
            Object::Dict(dict)
        })
    });

    group.bench_function("lookup", |b| {
        let dict = Object::Dict(small_dict.clone());
        let key = Name::new("Type");
        b.iter(|| {
            if let Object::Dict(ref d) = dict {
                black_box(d.get(black_box(&key)))
            } else {
                panic!()
            }
        })
    });

    group.bench_function("insert", |b| {
        b.iter_batched(
            HashMap::new,
            |mut dict: HashMap<Name, Object>| {
                dict.insert(Name::new(black_box("NewKey")), Object::Int(black_box(42)));
                dict
            },
            criterion::BatchSize::SmallInput,
        )
    });

    group.finish();
}

fn bench_object_type_checks(c: &mut Criterion) {
    let objects = vec![
        Object::Null,
        Object::Bool(true),
        Object::Int(42),
        Object::Real(PI),
        Object::Name(Name::new("Test")),
        Object::String(PdfString::new(b"Hello".to_vec())),
        Object::Array(vec![Object::Int(1), Object::Int(2)]),
        Object::Dict(HashMap::new()),
        Object::Ref(ObjRef::new(1, 0)),
    ];

    let mut group = c.benchmark_group("pdf/object/type_check");

    group.bench_function("is_null", |b| {
        b.iter(|| {
            for obj in &objects {
                black_box(obj.is_null());
            }
        })
    });

    group.bench_function("as_int", |b| {
        b.iter(|| {
            for obj in &objects {
                black_box(obj.as_int());
            }
        })
    });

    group.bench_function("as_dict", |b| {
        b.iter(|| {
            for obj in &objects {
                black_box(obj.as_dict());
            }
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_object_creation,
    bench_name_operations,
    bench_string_operations,
    bench_array_operations,
    bench_dict_operations,
    bench_object_type_checks,
);

criterion_main!(benches);
