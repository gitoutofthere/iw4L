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

fn main() {
    let args: Vec<i32> = std::env::args()
        .skip(1)
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
