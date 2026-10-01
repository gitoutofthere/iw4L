//! Native output witnesses through the public compiler and VM, without game data.
use std::collections::BTreeMap;
use std::sync::Mutex;
use sim::script::{Catalog, LevelData, Namespace, NativeRegistry, Program, Value};
use sim::{MatchBootstrap, SimWorld, StepReason, Tick, TickInput};

static OBSERVED: Mutex<Vec<Vec<Value>>> = Mutex::new(Vec::new());

const IW4: &str = r#"
main(large, maximum) {
    v = (large, large, 0);
    gettime("length", length(v));
    gettime("lengthsquared", lengthsquared(v));
    gettime("distance", distance((0,0,0), v));
    gettime("distancesquared", distancesquared((0,0,0), v));
    gettime("distance2d", distance2d((0,0,0), v));
    gettime("vectordot_nan", vectordot(v, (large,-large,0)));
    gettime("normalize_large", vectornormalize(v));
    gettime("nearest_nan", pointonsegmentnearesttopoint((0,0,0), v, v));
    gettime("line_nan", vectorfromlinetopoint((-maximum,0,0), (maximum,0,0), v));
    points = [];
    points[0] = (maximum,0,0);
    points[1] = (maximum,0,0);
    gettime("averagepoint", averagepoint(points));
    gettime("averagenormal", averagenormal(points));
    gettime("random_range", randomfloatrange(-maximum, maximum));
    gettime("angle_max", angleclamp(maximum));
    gettime("angle_negmax", angleclamp(-maximum));
    setdvar("native_probe_scalar", "1e39");
    setdvar("native_probe_vector", "1e39 0 0");
    gettime("dvar_float", getdvarfloat("native_probe_scalar"));
    gettime("dvar_vector", getdvarvector("native_probe_vector"));
    gettime("sqrt_negative", sqrt(-1));
    gettime("after_error", 1);
}
"#;
const T5: &str = r#"
main(large, maximum) {
    gettime("log_zero", log(0));
    gettime("log_negative", log(-1));
    gettime("cast_float", float("1e39"));
    gettime("cross_nan", vectorcross((large,large,0), (large,large,0)));
    gettime("rotate_overflow", rotatepoint((maximum,maximum,maximum), (45,45,0)));
    gettime("after_outputs", 1);
}
"#;

fn classify(value: &Value) -> String {
    let scalar = |f: f32| if f.is_nan() { "nan" } else if f == f32::INFINITY { "+inf" }
        else if f == f32::NEG_INFINITY { "-inf" } else { "finite" };
    match value {
        Value::Float(f) => scalar(*f).into(),
        Value::Vector(v) => v.iter().map(|f| scalar(*f)).collect::<Vec<_>>().join(","),
        Value::Undefined => "undefined".into(),
        Value::Int(n) => format!("int:{n}"),
        other => panic!("unexpected output {other:?}"),
    }
}

fn run(source: &str, catalog: Catalog, expected: &[(&str, &str)]) {
    OBSERVED.lock().unwrap().clear();
    let module = "probe/native";
    let sources = BTreeMap::from([(module.to_owned(), source.to_owned())]);
    let program = Program::load(&sources, &[module], &catalog).expect("compile native witnesses");
    let mut registry = NativeRegistry::default();
    // Only this observer is substituted. Every numeric builtin stays production code.
    registry.register(Namespace::Function, "gettime", |_, _, args| {
        OBSERVED.lock().unwrap().push(args.to_vec());
        Ok(Value::Undefined)
    });
    let mut world = SimWorld::new();
    world.bootstrap(MatchBootstrap::default()).expect("bootstrap");
    world.install_gsc_program(program, registry, LevelData::default()).expect("install");
    world.start_gsc(&format!("{module}::main"), Value::level(),
        vec![Value::Float(1e20), Value::Float(f32::MAX)]).expect("start");
    sim::try_step(&mut world, Tick(1), &TickInput::default(), 50,
        StepReason::AuthorityFrame).expect("step survives native outputs");
    assert!(world.take_script_fault().is_none(), "no terminal script fault");
    let observed = OBSERVED.lock().unwrap();
    assert_eq!(observed.len(), expected.len(), "all observers execute");
    for (row, &(name, wanted)) in observed.iter().zip(expected) {
        assert_eq!(row.len(), 2);
        assert_eq!(row[0], Value::string(name));
        let actual = classify(&row[1]);
        assert_eq!(actual, wanted, "{name}: {:?}", row[1]);
        println!("native_case={name} class={actual} value={:?}", row[1]);
    }
}

fn main() {
    run(IW4, Catalog::iw4(), &[
        ("length", "+inf"), ("lengthsquared", "+inf"), ("distance", "+inf"),
        ("distancesquared", "+inf"), ("distance2d", "+inf"), ("vectordot_nan", "nan"),
        ("normalize_large", "finite,finite,finite"), ("nearest_nan", "nan,nan,nan"),
        ("line_nan", "nan,nan,nan"), ("averagepoint", "+inf,finite,finite"),
        ("averagenormal", "nan,finite,finite"), ("random_range", "+inf"),
        ("angle_max", "finite"), ("angle_negmax", "finite"),
        ("dvar_float", "+inf"), ("dvar_vector", "+inf,finite,finite"),
        ("sqrt_negative", "undefined"), ("after_error", "int:1"),
    ]);
    run(T5, Catalog::t5(), &[
        ("log_zero", "-inf"), ("log_negative", "nan"), ("cast_float", "+inf"),
        ("cross_nan", "finite,finite,nan"), ("rotate_overflow", "finite,+inf,finite"),
        ("after_outputs", "int:1"),
    ]);
    println!("native_finite_probe=ok iw4=18 t5=6 public_vm=true terminal_fault=false");
}
