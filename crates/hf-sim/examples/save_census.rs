//! Bytes each part of a save's `world.bin` takes, to find what grows over a long game (#431).
//! Top-level Snapshot fields, then fields summed over all agents (dead ones included), then
//! the largest second-level fields. usage: save_census <dir.hfsave> [more saves...]
use std::collections::BTreeMap;

use serde::de::{Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};

/// A MessagePack value with any key type (the world's maps are keyed by tiles and ids).
#[derive(PartialEq)]
enum Value {
    Nil,
    Bool(bool),
    Num(u8),
    Str(String),
    Bin(usize),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}

impl Value {
    fn get(&self, k: &str) -> Option<&Value> {
        match self {
            Value::Object(o) => o.iter().find(|e| e.0 == k).map(|e| &e.1),
            _ => None,
        }
    }
}

struct V;
impl<'de> Visitor<'de> for V {
    type Value = Value;
    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("any")
    }
    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Nil)
    }
    fn visit_none<E>(self) -> Result<Value, E> {
        Ok(Value::Nil)
    }
    fn visit_some<D: Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
        Value::deserialize(d)
    }
    fn visit_bool<E>(self, b: bool) -> Result<Value, E> {
        Ok(Value::Bool(b))
    }
    fn visit_u64<E>(self, x: u64) -> Result<Value, E> {
        Ok(Value::Num(if x < 128 {
            1
        } else if x < 65536 {
            3
        } else if x < 1 << 32 {
            5
        } else {
            9
        }))
    }
    fn visit_i64<E>(self, x: i64) -> Result<Value, E> {
        Ok(Value::Num(if (-32..128).contains(&x) {
            1
        } else if x.abs() < 32768 {
            3
        } else {
            5
        }))
    }
    fn visit_f32<E>(self, _: f32) -> Result<Value, E> {
        Ok(Value::Num(5))
    }
    fn visit_f64<E>(self, _: f64) -> Result<Value, E> {
        Ok(Value::Num(9))
    }
    fn visit_str<E>(self, s: &str) -> Result<Value, E> {
        Ok(Value::Str(s.to_string()))
    }
    fn visit_string<E>(self, s: String) -> Result<Value, E> {
        Ok(Value::Str(s))
    }
    fn visit_bytes<E>(self, b: &[u8]) -> Result<Value, E> {
        Ok(Value::Bin(b.len()))
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
        let mut v = Vec::new();
        while let Some(x) = a.next_element::<Value>()? {
            v.push(x);
        }
        Ok(Value::Array(v))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut a: A) -> Result<Value, A::Error> {
        let mut v = Vec::new();
        while let Some((k, x)) = a.next_entry::<Value, Value>()? {
            let k = match k {
                Value::Str(s) => s,
                other => format!("#{}", size(&other)),
            };
            v.push((k, x));
        }
        Ok(Value::Object(v))
    }
}
impl<'de> Deserialize<'de> for Value {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Value, D::Error> {
        d.deserialize_any(V)
    }
}

/// Approximate encoded bytes.
fn size(v: &Value) -> usize {
    match v {
        Value::Nil | Value::Bool(_) => 1,
        Value::Num(n) => *n as usize,
        Value::Str(s) => s.len() + 2,
        Value::Bin(n) => n + 5,
        Value::Array(a) => 3 + a.iter().map(size).sum::<usize>(),
        Value::Object(o) => 3 + o.iter().map(|(k, x)| k.len() + 2 + size(x)).sum::<usize>(),
    }
}

fn len(v: &Value) -> String {
    match v {
        Value::Array(a) => a.len().to_string(),
        Value::Object(o) => format!("{{{}}}", o.len()),
        _ => String::new(),
    }
}

fn main() -> anyhow::Result<()> {
    for dir in std::env::args().skip(1) {
        let bytes = std::fs::read(std::path::Path::new(&dir).join("world.bin"))?;
        let v: Value = rmp_serde::from_slice(&bytes)?;
        println!("== {dir}: {} bytes", bytes.len());
        let Value::Object(top) = &v else { continue };
        let mut rows: Vec<(usize, &String, &Value)> =
            top.iter().map(|(k, x)| (size(x), k, x)).collect();
        rows.sort_by_key(|r| std::cmp::Reverse(r.0));
        for (n, k, x) in rows.iter().take(25) {
            println!("{n:>10}  {k} {}", len(x));
        }
        if let Some(Value::Array(agents)) = v.get("agents") {
            let alive = agents
                .iter()
                .filter(|a| a.get("alive") == Some(&Value::Bool(true)))
                .count();
            println!(
                "-- agents: {} ({} alive); per-field bytes summed",
                agents.len(),
                alive
            );
            let mut per: BTreeMap<String, (usize, usize)> = BTreeMap::new();
            for a in agents {
                if let Value::Object(o) = a {
                    for (k, x) in o {
                        let e = per.entry(k.clone()).or_default();
                        e.0 += size(x);
                        if let Value::Array(arr) = x {
                            e.1 += arr.len();
                        }
                        if let Value::Object(sub) = x {
                            for (k2, y) in sub {
                                let e = per.entry(format!("{k}.{k2}")).or_default();
                                e.0 += size(y);
                                if let Value::Array(arr) = y {
                                    e.1 += arr.len();
                                }
                            }
                        }
                    }
                }
            }
            let mut per: Vec<_> = per.into_iter().collect();
            per.sort_by_key(|r| std::cmp::Reverse(r.1 .0));
            for (k, (n, l)) in per.iter().take(30) {
                println!("{n:>10}  agents[].{k} (elems {l})");
            }
        }
        // Second level of every other field.
        let mut second: Vec<(usize, String, String)> = Vec::new();
        for (k, x) in top {
            if k == "agents" {
                continue;
            }
            if let Value::Object(o) = x {
                for (k2, y) in o {
                    second.push((size(y), format!("{k}.{k2}"), len(y)));
                }
            }
        }
        second.sort_by_key(|r| std::cmp::Reverse(r.0));
        println!("-- largest nested fields");
        for (n, k, l) in second.iter().take(20) {
            println!("{n:>10}  {k} {l}");
        }
    }
    Ok(())
}
