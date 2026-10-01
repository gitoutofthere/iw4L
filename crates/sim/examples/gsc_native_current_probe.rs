mod witnesses {
//! Native output witnesses through the public compiler and VM, without game data.
use std::collections::BTreeMap;
use std::sync::Mutex;
use sim::script::{Catalog, KeyType, LevelData, Namespace, NativeRegistry, Program, Value};
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
    gettime("map_origin", level.struct[0].origin);
    gettime("map_radius", level.struct[0].radius);
    gettime("map_custom", level.struct[0].probe_float);
    e = spawn("script_origin", (0,0,0));
    e.angles = (maximum,0,0);
    e addpitch(maximum);
    gettime("native_side_effect_field", e.angles);
    gettime("native_side_effect_getter", e gettagangles("tag_origin"));
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
            ("origin".into(), "1e39 0 0".into()),
            ("radius".into(), "1e39".into()),
            ("probe_float".into(), "1e39".into()),
        ]],
        keys: BTreeMap::from([("probe_float".into(), KeyType::Float)]),
        ..Default::default()
    };
    world.install_gsc_program(program, registry, level).expect("install");
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
        let guarded = classify(&observed[0][1]) == "undefined";
        let wanted = if guarded && (wanted.contains("inf") || wanted.contains("nan")) { "undefined" } else { wanted };
        assert_eq!(actual, wanted, "{name}: {:?}", row[1]);
        println!("native_case={name} class={actual} value={:?}", row[1]);
    }
    let guarded = classify(&observed[0][1]) == "undefined";
    drop(observed);
    if catalog.realm() == sim::script::Realm::Iw4 {
        OBSERVED.lock().unwrap().clear();
        world.start_gsc(&format!("{module}::bad_argument"), Value::level(), vec![Value::Float(f32::NAN)]).expect("start bad engine argument");
        sim::try_step(&mut world, Tick(2), &TickInput::default(), 50,
            StepReason::AuthorityFrame).expect("bad local does not terminate match");
        assert!(world.take_script_fault().is_none());
        let observed = OBSERVED.lock().unwrap();
        assert_eq!(observed.len(), 2);
        assert_eq!(classify(&observed[0][1]), if guarded { "undefined" } else { "nan" });
        assert_eq!(observed[1][1], Value::Int(1));
        println!("native_argument_probe=ok guarded={guarded} terminal_fault=false");
    }
}

pub fn run() {
    run(IW4, Catalog::iw4(), &[
        ("length", "+inf"), ("lengthsquared", "+inf"), ("distance", "+inf"),
        ("distancesquared", "+inf"), ("distance2d", "+inf"), ("vectordot_nan", "nan"),
        ("normalize_large", "finite,finite,finite"), ("nearest_nan", "nan,nan,nan"),
        ("line_nan", "nan,nan,nan"), ("averagepoint", "+inf,finite,finite"),
        ("averagenormal", "nan,finite,finite"), ("random_range", "+inf"),
        ("angle_max", "finite"), ("angle_negmax", "finite"),
        ("dvar_float", "+inf"), ("dvar_vector", "+inf,finite,finite"),
        ("map_origin", "+inf,finite,finite"), ("map_radius", "+inf"), ("map_custom", "+inf"),
        ("native_side_effect_field", "+inf,finite,finite"),
        ("native_side_effect_getter", "+inf,finite,finite"),
        ("sqrt_negative", "undefined"), ("after_error", "int:1"),
    ]);
    run(T5, Catalog::t5(), &[
        ("log_zero", "-inf"), ("log_negative", "nan"), ("cast_float", "+inf"),
        ("cross_nan", "finite,finite,nan"), ("rotate_overflow", "finite,+inf,finite"),
        ("after_outputs", "int:1"),
    ]);
    println!("native_finite_probe=ok iw4=23 t5=6 public_vm=true terminal_fault=false");
}

}
mod cost {
//! Cost of an authority step spent in the GSC scheduler, with no game data.
//!
//! A synthetic module shaped like the multiplayer scripts: per player, event
//! listeners parked on `waittill` under two `endon`s, and tickers that wait a
//! frame, call a few functions deep and leave an array behind; one level thread
//! notifies every player each frame; a level array of structs stands in for
//! the map's `level.struct` that every heap collection walks. Everything goes
//! through the public `sim` API, so it measures what `try_step` measures.
//!
//! ```text
//! cargo run -p sim --example gsc_vm_bench --profile play -- [players] [listeners] [tickers] [depth] [structs] [ticks] [deaths_every]
//! ```
//!
//! Defaults: 16 players, 40 listeners and 4 tickers each, depth 4, 2000 structs,
//! 1200 ticks.
//! The `gsc census` lines on stderr say what the scheduler walked.

use std::collections::BTreeMap;
use std::time::Instant;

use sim::script::{Catalog, LevelData, NativeRegistry, Program, Value};
use sim::{MatchBootstrap, SimWorld, StepReason, Tick, TickInput};

const MODULE: &str = "bench/mp";

const SOURCE: &str = r#"
main( players, listeners, tickers, depth, structs, deaths )
{
    level.deaths = deaths;
    level.listeners = listeners;
    level.depth = depth;
    level.structs = [];
    for ( i = 0; i < structs; i++ )
    {
        s = spawnstruct();
        s.origin = ( i, 0, 0 );
        s.targetname = "struct" + i;
        s.script_noteworthy = i;
        level.structs[ level.structs.size ] = s;
    }
    level.players = [];
    for ( i = 0; i < players; i++ )
    {
        player = spawnstruct();
        player.counter = 0;
        level.players[ level.players.size ] = player;
        for ( j = 0; j < listeners; j++ )
            player thread listen( "event" + ( j % 8 ) );
        for ( j = 0; j < tickers; j++ )
            player thread tick( depth );
        player thread lives();
    }
    level thread drive();
}

listen( name )
{
    self endon( "disconnect" );
    self endon( "death" );
    for ( ;; )
    {
        self waittill( name, value );
        self.counter = self.counter + value;
    }
}

tick( depth )
{
    self endon( "disconnect" );
    for ( ;; )
    {
        wait 0.05;
        self.counter = self.counter + nest( depth );
        scratch = [];
        scratch[ 0 ] = self.counter;
        scratch[ 1 ] = gettime();
    }
}

nest( depth )
{
    if ( depth <= 0 )
        return 1;
    return nest( depth - 1 ) + 1;
}

lives()
{
    if ( level.deaths <= 0 )
        return;
    for ( j = 0; j < 8; j++ )
        self thread life();
}

life()
{
    self endon( "death" );
    for ( ;; )
    {
        wait 0.05;
        self.counter = self.counter + nest( level.depth );
    }
}

drive()
{
    frame = 0;
    victim = 0;
    for ( ;; )
    {
        wait 0.05;
        foreach ( player in level.players )
            player notify( "event" + randomint( 8 ), 1 );
        frame = frame + 1;
        if ( level.deaths > 0 && frame % level.deaths == 0 )
        {
            player = level.players[ victim % level.players.size ];
            victim = victim + 1;
            player notify( "death" );
            for ( j = 0; j < level.listeners; j++ )
                player thread listen( "event" + ( j % 8 ) );
            player thread lives();
        }
    }
}
"#;

pub fn run() {
    let args: Vec<i32> = std::env::args()
        .skip(2)
        .map(|arg| arg.parse().expect("arguments are integers"))
        .collect();
    let arg = |index: usize, default: i32| args.get(index).copied().unwrap_or(default);
    let (players, listeners, tickers, depth, structs, ticks, deaths) = (
        arg(0, 16),
        arg(1, 40),
        arg(2, 4),
        arg(3, 4),
        arg(4, 2000),
        arg(5, 1200),
        arg(6, 0),
    );

    let sources = BTreeMap::from([(MODULE.to_owned(), SOURCE.to_owned())]);
    let program = Program::load(&sources, &[MODULE], &Catalog::iw4())
        .unwrap_or_else(|fault| panic!("compile: {fault}"));
    let mut world = SimWorld::new();
    world
        .bootstrap(MatchBootstrap::default())
        .unwrap_or_else(|error| panic!("bootstrap: {error}"));
    world
        .install_gsc_program(program, NativeRegistry::default(), LevelData::default())
        .unwrap_or_else(|fault| panic!("install: {fault}"));
    world
        .start_gsc(
            &format!("{MODULE}::main"),
            Value::level(),
            vec![
                Value::Int(players),
                Value::Int(listeners),
                Value::Int(tickers),
                Value::Int(depth),
                Value::Int(structs),
                Value::Int(deaths),
            ],
        )
        .unwrap_or_else(|fault| panic!("start: {fault}"));

    let input = TickInput::default();
    let mut samples = Vec::with_capacity(ticks.max(2) as usize);
    for tick in 1..=ticks.max(2) as u32 {
        let started = Instant::now();
        sim::try_step(
            &mut world,
            Tick(tick),
            &input,
            50,
            StepReason::AuthorityFrame,
        )
        .unwrap_or_else(|fault| panic!("tick {tick}: {fault}"));
        samples.push(started.elapsed().as_secs_f64() * 1e3);
    }

    // The first tick runs `main` and parks every thread.
    let mut steady = samples[1..].to_vec();
    steady.sort_by(f64::total_cmp);
    let mean = steady.iter().sum::<f64>() / steady.len() as f64;
    let at = |p: f64| steady[((steady.len() - 1) as f64 * p) as usize];
    println!(
        "players={players} listeners={listeners} tickers={tickers} depth={depth} structs={structs} deaths_every={deaths} ticks={}",
        samples.len()
    );
    println!(
        "first {:.3} ms | steady mean {:.3} ms  p50 {:.3}  p95 {:.3}  max {:.3}",
        samples[0],
        mean,
        at(0.5),
        at(0.95),
        at(1.0)
    );
}

}
fn main() {
 match std::env::args().nth(1).as_deref() {
 Some("probe") => witnesses::run(),
 Some("bench") => cost::run(),
 _ => panic!("expected probe or bench"),
 }
}
