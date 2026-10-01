use std::collections::BTreeMap;
use std::sync::Mutex;
use sim::script::{Catalog, LevelData, Namespace, NativeRegistry, Program, Value};
use sim::{MatchBootstrap, SimWorld, StepReason, Tick, TickInput};
static OBSERVED: Mutex<Vec<Vec<Value>>> = Mutex::new(Vec::new());
const SOURCE: &str = r#"
finished(name) { self waittill("movedone"); gettime(name, 1); }
main(maximum) {
    late = spawn("script_origin", (0,0,0));
    late thread finished("late_done");
    late moveto((maximum,0,0), 1.0, 0.5, 0.5);
    normal = spawn("script_origin", (0,0,0));
    normal thread finished("normal_done");
    normal moveto((100,0,0), 1.0, 0.5, 0.5);
    wait 0.25;
    gettime("before_bad_tick", late.origin);
    wait 0.05;
    gettime("after_bad_tick", late.origin);
    wait 1.0;
    gettime("after_cancel", late.origin);
    gettime("normal_pose", normal.origin);
    gettime("after", 1);
}
"#;
fn main() {
    let program=Program::load(&BTreeMap::from([("probe/motion".to_owned(),SOURCE.to_owned())]),&["probe/motion"],&Catalog::iw4()).expect("compile");
    let mut registry=NativeRegistry::default();
    registry.register(Namespace::Function,"gettime",|_,_,args| { OBSERVED.lock().unwrap().push(args.to_vec()); Ok(Value::Undefined) });
    let mut world=SimWorld::new();world.bootstrap(MatchBootstrap::default()).expect("bootstrap");
    world.install_gsc_program(program,registry,LevelData::default()).expect("install");
    world.start_gsc("probe/motion::main",Value::level(),vec![Value::Float(f32::MAX)]).expect("start");
    for tick in 1..=28 { sim::try_step(&mut world,Tick(tick),&TickInput::default(),50,StepReason::AuthorityFrame).expect("step"); }
    assert!(world.take_script_fault().is_none());
    let rows=OBSERVED.lock().unwrap();
    let find=|name:&str|rows.iter().find(|r|r[0]==Value::string(name)).map(|r|r[1].clone());
    let last=Value::Vector([f32::MAX*0.125,0.0,0.0]);
    for name in ["before_bad_tick","after_bad_tick","after_cancel"] { assert_eq!(find(name),Some(last.clone()),"retains last finite250ms pose: {name}"); }
    assert_eq!(find("late_done"),None);
    assert_eq!(find("normal_done"),Some(Value::Int(1)));
    assert_eq!(find("normal_pose"),Some(Value::Vector([100.0,0.0,0.0])));
    assert_eq!(find("after"),Some(Value::Int(1)));assert_eq!(rows.len(),6);
    for row in rows.iter() { println!("motion_case={:?} value={:?}",row[0],row[1]); }
    println!("future_motion_probe=ok late_velocity_overflow_ms=300 last_pose_ms=250 normal_completion=true false_completion=false ticks=28 terminal_fault=false");
}
