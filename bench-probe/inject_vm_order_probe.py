#!/usr/bin/env python3
"""Inject a disposable internal VM probe into a benchmark-only checkout."""
from pathlib import Path
import sys

repo = Path(sys.argv[1])
here = Path(__file__).resolve().parent
runtime = repo / "crates/sim/src/script/runtime/mod.rs"
source = runtime.read_text()
assert "__iw4l_thread_order_probe" not in source
if "fn take_thread(" in source:
    helpers = """
fn __probe_take(world: &mut World, entity: Entity) -> Thread { take_thread(world, entity) }
fn __probe_put(world: &mut World, entity: Entity, thread: Thread) { put_thread(world, entity, thread); }
"""
else:
    helpers = """
fn __probe_take(world: &mut World, entity: Entity) -> Thread { world.entity_mut(entity).take::<Thread>().unwrap() }
fn __probe_put(world: &mut World, entity: Entity, thread: Thread) { world.entity_mut(entity).insert(thread); }
"""
runtime.write_text(source + "\n" + (here / "vm_thread_order_probe.rs").read_text() + helpers)
script = repo / "crates/sim/src/script/mod.rs"
script.write_text(script.read_text() + "\npub use runtime::__iw4l_thread_order_probe;\n")
example = repo / "crates/sim/examples/gsc_thread_order_probe.rs"
example.parent.mkdir(parents=True, exist_ok=True)
example.write_text("fn main() { sim::script::__iw4l_thread_order_probe(); }\n")
