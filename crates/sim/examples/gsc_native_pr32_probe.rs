//! Native output witnesses through the public compiler and VM, without game data.
use std::collections::BTreeMap;
use std::sync::Mutex;
use sim::script::{Catalog, KeyType, LevelData, Namespace, NativeRegistry, Program, Value};
use sim::{MatchBootstrap, SimWorld, StepReason, Tick, TickInput};

static OBSERVED: Mutex<Vec<Vec<Value>>> = Mutex::new(Vec::new());

const IW4: &str = r#"
main(large, maximum, badvector) {
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
    gettime("map_origin", level.struct[0].origin);
    gettime("map_radius", level.struct[0].radius);
    gettime("map_custom", level.struct[0].probe_float);
    e = spawn("script_origin", (0,0,0));
    e.angles = (maximum,0,0);
    e addpitch(maximum);
    gettime("native_side_effect_field", e.angles);
    gettime("native_side_effect_getter", e gettagangles("tag_origin"));
    e.origin = badvector;
    gettime("named_origin", e.origin);
    e = spawn("script_origin", (0,0,0));
    e["origin"] = badvector;
    gettime("indexed_origin", e.origin);
    e = spawn("script_origin", (0,0,0));
    e moveto((maximum,0,0), 0.05);
    wait 0.05;
    gettime("motion_origin", e.origin);
    gettime("sqrt_negative", sqrt(-1));
    gettime("after_error", 1);
}
bad_argument(value) {
    gettime("bad_argument", value);
    gettime("after_bad_argument", 1);
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
    let sources = BTreeMap::from([
        (module.to_owned(), source.to_owned()),
        ("codescripts/struct".to_owned(), "initstructs() { level.struct = []; }".to_owned()),
    ]);
    let program = Program::load(&sources, &[module, "codescripts/struct"], &catalog).expect("compile native witnesses");
    let mut registry = NativeRegistry::default();
    // Only this observer is substituted. Every numeric builtin stays production code.
    registry.register(Namespace::Function, "gettime", |_, _, args| {
        OBSERVED.lock().unwrap().push(args.to_vec());
        Ok(Value::Undefined)
    });
    let mut world = SimWorld::new();
    world.bootstrap(MatchBootstrap::default()).expect("bootstrap");
    let level = LevelData {
        entities: vec![vec![
            ("classname".into(), "script_struct".into()),
            ("origin".into(), "1 2 3".into()),
            ("radius".into(), "42".into()),
            ("probe_float".into(), "42".into()),
        ]],
        keys: BTreeMap::from([("probe_float".into(), KeyType::Float)]),
        ..Default::default()
    };
    world.install_gsc_program(program, registry, level).expect("install");
    world.start_gsc(&format!("{module}::main"), Value::level(),
        vec![Value::Float(1e20), Value::Float(f32::MAX), Value::Vector([f32::INFINITY,0.0,0.0])]).expect("start");
    for tick in 1..=3 {
        sim::try_step(&mut world, Tick(tick), &TickInput::default(), 50,
            StepReason::AuthorityFrame).expect("step survives native outputs");
    }
    assert!(world.take_script_fault().is_none(), "no terminal script fault");
    let observed = OBSERVED.lock().unwrap();
    assert_eq!(observed.len(), expected.len(), "all observers execute");
    for (row, &(name, wanted)) in observed.iter().zip(expected) {
        assert_eq!(row.len(), 2);
        assert_eq!(row[0], Value::string(name));
        let actual = classify(&row[1]);
        let guarded = classify(&observed[0][1]) == "undefined";
        let protected_pose = matches!(name, "native_side_effect_field" | "native_side_effect_getter" | "named_origin" | "indexed_origin" | "motion_origin");
        let wanted = if guarded && protected_pose { "finite,finite,finite" }
            else if guarded && (wanted.contains("inf") || wanted.contains("nan")) { "undefined" } else { wanted };
        assert_eq!(actual, wanted, "{name}: {:?}", row[1]);
        if name=="motion_origin" {
            assert_eq!(row[1],Value::Vector([if guarded {0.0} else {f32::MAX},0.0,0.0]), "invalid velocity never publishes partial motion");
        }
        println!("native_case={name} class={actual} value={:?}", row[1]);
    }
    let guarded = classify(&observed[0][1]) == "undefined";
    drop(observed);
    if catalog.realm() == sim::script::Realm::Iw4 {
        OBSERVED.lock().unwrap().clear();
        world.start_gsc(&format!("{module}::bad_argument"), Value::level(), vec![Value::Float(f32::NAN)]).expect("start bad engine argument");
        sim::try_step(&mut world, Tick(4), &TickInput::default(), 50,
            StepReason::AuthorityFrame).expect("bad local does not terminate match");
        assert!(world.take_script_fault().is_none());
        let observed = OBSERVED.lock().unwrap();
        assert_eq!(observed.len(), if guarded {1} else {2});
        if !guarded {assert_eq!(classify(&observed[0][1]), "nan");}
        assert_eq!(observed.last().unwrap()[1], Value::Int(1));
        println!("native_argument_probe=ok guarded={guarded} terminal_fault=false");
    }
}

fn map_probe() {
    let module="probe/maps";
    let program=Program::load(&BTreeMap::from([(module.to_owned(), "main() {}".to_owned()),
        ("codescripts/struct".to_owned(), "initstructs() { level.struct = []; }".to_owned())]),
        &[module,"codescripts/struct"],&Catalog::iw4()).expect("compile maps");
    let guarded=include_str!("../src/script/host/entities.rs").contains("ensure_finite()");
    for key in ["origin","angles","radius","probe_float"] {
        let mut world=SimWorld::new();world.bootstrap(MatchBootstrap::default()).expect("bootstrap map");
        let level=LevelData{entities:vec![vec![("classname".into(),"script_struct".into()),
            (key.into(),if matches!(key,"origin"|"angles") {"1e39 0 0".into()} else {"1e39".into()})]],
            keys:BTreeMap::from([("probe_float".into(),KeyType::Float)]),..Default::default()};
        let rejected=world.install_gsc_program(program.clone(),NativeRegistry::default(),level).is_err();
        assert_eq!(rejected,guarded,"typed map narrowing {key}");
        println!("map_case key={key} rejected={rejected}");
    }
}
fn main() {
    map_probe();
    run(IW4, Catalog::iw4(), &[
        ("length", "+inf"), ("lengthsquared", "+inf"), ("distance", "+inf"),
        ("distancesquared", "+inf"), ("distance2d", "+inf"), ("vectordot_nan", "nan"),
        ("normalize_large", "finite,finite,finite"), ("nearest_nan", "nan,nan,nan"),
        ("line_nan", "nan,nan,nan"), ("averagepoint", "+inf,finite,finite"),
        ("averagenormal", "nan,finite,finite"), ("random_range", "+inf"),
        ("angle_max", "finite"), ("angle_negmax", "finite"),
        ("dvar_float", "+inf"), ("dvar_vector", "+inf,finite,finite"),
        ("map_origin", "finite,finite,finite"), ("map_radius", "finite"), ("map_custom", "finite"),
        ("native_side_effect_field", "+inf,finite,finite"),
        ("native_side_effect_getter", "+inf,finite,finite"),
        ("named_origin", "+inf,finite,finite"), ("indexed_origin", "+inf,finite,finite"),
        ("motion_origin", "finite,finite,finite"),
        ("sqrt_negative", "undefined"), ("after_error", "int:1"),
    ]);
    run(T5, Catalog::t5(), &[
        ("log_zero", "-inf"), ("log_negative", "nan"), ("cast_float", "+inf"),
        ("cross_nan", "finite,finite,nan"), ("rotate_overflow", "finite,+inf,finite"),
        ("after_outputs", "int:1"),
    ]);
    println!("native_finite_probe=ok iw4=26 t5=6 maps=4 public_vm=true terminal_fault=false");
}
