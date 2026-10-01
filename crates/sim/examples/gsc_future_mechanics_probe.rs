use std::collections::BTreeMap;
use std::sync::Mutex;
use sim::script::{Catalog, LevelData, Namespace, NativeRegistry, Program, Value};
use sim::{MatchBootstrap, SimWorld, StepReason, Tick, TickInput};
static OBSERVED: Mutex<Vec<Vec<Value>>> = Mutex::new(Vec::new());
const SOURCE: &str = r#"
class(v, maximum) {
    if (v[0] != v[0] || v[1] != v[1] || v[2] != v[2]) return "nan";
    if (v[0] > maximum || v[1] > maximum || v[2] > maximum) return "inf";
    if (v[0] < -maximum || v[1] < -maximum || v[2] < -maximum) return "inf";
    return "finite";
}
finished(name) {
    self waittill("physics_finished");
    gettime(name, 1);
}
main(maximum) {
    bad = spawn("script_origin", (maximum,0,0));
    bad thread finished("bad_done");
    bad physicslaunchserver((0,0,0), (maximum,0,0));
    good = spawn("script_origin", (0,0,0));
    good thread finished("good_done");
    good physicslaunchclient((0,0,0), (100,0,0));
    parent = spawn("script_origin", (0,0,0));
    child = spawn("script_origin", (0,0,0));
    child linkto(parent, "tag_origin", (maximum,0,0));
    normal = spawn("script_origin", (0,0,0));
    normal linkto(parent, "tag_origin", (10,0,0));
    wait 0.05;
    gettime("body", class(bad.origin, maximum));
    gettime("good_body", good.origin);
    gettime("link_before", class(child.origin, maximum));
    parent.origin = (maximum,0,0);
    wait 0.05;
    gettime("link_overflow", class(child.origin, maximum));
    parent.origin = (-maximum,0,0);
    wait 0.05;
    gettime("link_detached", child.origin == (maximum,0,0));
    parent.origin = (20,0,0);
    wait 0.05;
    gettime("good_link", normal.origin);
    wait 10.0;
    gettime("after", 1);
}
"#;
fn main() {
    let sources = BTreeMap::from([("probe/future".to_owned(),SOURCE.to_owned())]);
    let program=Program::load(&sources,&["probe/future"],&Catalog::iw4()).expect("compile");
    let mut registry=NativeRegistry::default();
    registry.register(Namespace::Function,"gettime",|_,_,args| {
        OBSERVED.lock().unwrap().push(args.to_vec()); Ok(Value::Undefined)
    });
    let mut world=SimWorld::new();
    world.bootstrap(MatchBootstrap::default()).expect("bootstrap");
    world.install_gsc_program(program,registry,LevelData::default()).expect("install");
    world.start_gsc("probe/future::main",Value::level(),vec![Value::Float(f32::MAX)]).expect("start");
    for tick in 1..=205 { sim::try_step(&mut world,Tick(tick),&TickInput::default(),50,StepReason::AuthorityFrame).expect("step"); }
    assert!(world.take_script_fault().is_none());
    let rows=OBSERVED.lock().unwrap();
    for row in rows.iter() { println!("future_case={:?} value={:?}",row[0],row[1]); }
    let find=|name:&str| rows.iter().find(|r|r[0]==Value::string(name)).map(|r|r[1].clone());
    let guarded=include_str!("../src/script/host/mechanics.rs").contains("non-finite body sample");
    assert_eq!(find("body"),Some(Value::string(if guarded {"finite"} else {"nan"})));
    assert_eq!(find("link_before"),Some(Value::string("finite")));
    assert_eq!(find("link_overflow"),Some(Value::string(if guarded {"finite"} else {"inf"})));
    assert_eq!(find("link_detached"),Some(Value::Int(i32::from(guarded))));
    assert_eq!(find("good_body"),Some(Value::Vector([5.0,0.0,-2.0])));
    assert_eq!(find("good_link"),Some(Value::Vector([30.0,0.0,0.0])));
    assert_eq!(find("good_done"),Some(Value::Int(1)));
    assert_eq!(find("bad_done").is_some(),!guarded);
    assert_eq!(find("after"),Some(Value::Int(1)));
    assert_eq!(rows.len(),if guarded {8} else {9});
    println!("future_mechanics_probe=ok guarded={guarded} ticks=205 finite_input=true stock_producers=true terminal_fault=false");
}
