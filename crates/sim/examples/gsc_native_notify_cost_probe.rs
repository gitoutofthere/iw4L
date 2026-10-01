mod notify {
//! Disposable VM native-success / pending-notify probe; no stock producer claim.
use std::collections::BTreeMap;
use std::sync::Mutex;
use sim::script::{Catalog, LevelData, Namespace, NativeRegistry, Program, Value};
use sim::{MatchBootstrap, SimWorld, StepReason, Tick, TickInput};
static OBS: Mutex<Vec<Vec<Value>>> = Mutex::new(Vec::new());
const SOURCE: &str = r#"
main(mode, numeric) {
    gettime("before", 1);
    if (mode == "wait") {
        level thread listener();
        wait 0.05;
    }
    if (mode == "root") {
        level endon("probe");
        getdvarfloat(mode, numeric);
        gettime("forbidden", 1);
    } else if (mode == "child" || mode == "nested" || mode == "multi") {
        gettime("child_result", child(mode, numeric));
    } else {
        result = getdvarfloat(mode, numeric);
        gettime("result", result);
    }
    gettime("after", 1);
}
child(mode, numeric) {
    level endon("probe");
    if (mode == "nested")
        return 321 + inner(mode, numeric);
    return 123 + getdvarfloat(mode, numeric);
}
inner(mode, numeric) { return 456 + getdvarfloat(mode, numeric); }
listener() { level waittill("probe"); gettime("listener", 1); }
"#;
fn observer(_: &mut bevy_ecs::world::World, _: &Value, args: &[Value]) -> Result<Value, String> {
    OBS.lock().unwrap().push(args.to_vec());
    Ok(Value::Undefined)
}
fn emitter(world: &mut bevy_ecs::world::World, _: &Value, args: &[Value]) -> Result<Value, String> {
    let Value::String(mode) = &args[0] else { panic!("mode"); };
    let Value::Int(numeric) = args[1] else { panic!("numeric"); };
    if &**mode == "multi" { sim::script::__iw4l_notify_probe_raise(world, "other"); }
    if &**mode != "plain" { sim::script::__iw4l_notify_probe_raise(world, "probe"); }
    if &**mode == "multi" { sim::script::__iw4l_notify_probe_raise(world, "probe"); }
    Ok(Value::Float(match numeric { 0 => 3.0, 1 => f32::NAN, 2 => f32::INFINITY, 3 => f32::NEG_INFINITY, _ => panic!("numeric") }))
}
fn class(value: &Value) -> &'static str {
    match value {
        Value::Undefined => "undefined",
        Value::Float(v) if v.is_nan() => "nan",
        Value::Float(v) if *v == f32::INFINITY => "inf",
        Value::Float(v) if *v == f32::NEG_INFINITY => "neginf",
        Value::Float(v) if *v == 3.0 => "finite3",
        other => panic!("unexpected observed value {other:?}"),
    }
}
pub fn run() {
    let module = "probe/notify";
    let program = Program::load(&BTreeMap::from([(module.to_owned(), SOURCE.to_owned())]), &[module], &Catalog::iw4()).expect("compile notify probe");
    for mode in ["plain", "wait", "root", "child", "nested", "multi"] {
        for numeric in 0..4 {
            OBS.lock().unwrap().clear();
            let mut registry = NativeRegistry::default();
            registry.register(Namespace::Function, "gettime", observer);
            registry.register(Namespace::Function, "getdvarfloat", emitter);
            let mut world = SimWorld::new();
            world.bootstrap(MatchBootstrap::default()).expect("bootstrap");
            world.install_gsc_program(program.clone(), registry, LevelData::default()).expect("install");
            world.start_gsc(&format!("{module}::main"), Value::level(), vec![Value::string(mode), Value::Int(numeric)]).expect("start");
            for tick in 1..=3 {
                sim::try_step(&mut world, Tick(tick), &TickInput::default(), 50, StepReason::AuthorityFrame).expect("step");
                assert!(world.take_script_fault().is_none(), "nonterminal {mode}/{numeric}");
            }
            let observed = OBS.lock().unwrap();
            let labels: Vec<_> = observed.iter().map(|row| {
                assert_eq!(row.len(), 2);
                let Value::String(label) = &row[0] else { panic!("observer label corrupted {row:?}"); };
                label.to_string()
            }).collect();
            let wanted: Vec<&str> = match mode {
                "plain" => vec!["before", "result", "after"],
                "wait" => vec!["before", "result", "after", "listener"],
                "root" => vec!["before"],
                _ => vec!["before", "child_result", "after"],
            };
            assert_eq!(labels, wanted, "event/stack order {mode}/{numeric}");
            let result = match mode {
                "plain" | "wait" => class(&observed[1][1]),
                "root" => "root-ended",
                _ => { assert_eq!(observed[1][1], Value::Undefined); "child-ended" }
            };
            if numeric == 0 && matches!(mode, "plain" | "wait") { assert_eq!(result, "finite3"); }
            println!("notify_case mode={mode} numeric={numeric} labels={} result={result} terminal=false", labels.join(","));
        }
    }
    println!("notify_probe=ok cases=24 synthetic_native=true production_compiler_vm=true");
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

pub fn entry() {
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
fn main() { match std::env::args().nth(1).as_deref() { Some("probe") => notify::run(), Some("bench") => cost::entry(), _ => panic!("expected probe or bench") } }
