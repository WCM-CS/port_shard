# port_shard

Fast_Vector || Port_Shard
- Stores the first 16 elements on the stack and the remainder on the heap.

Public Api

Builders 
- new()
- from_vec()

Accessors
- insert()
- contains()
- as_slice()

```rust

let mut ports: Chimera<u16> = Chimera::new();

for i in 1..=4 {
    ports.insert(i);
}

assert!(ports.contains(&1));
assert_eq(ports.as_slice(), &[1,2,3,4]);


let fast_vec: Chimera<&str> = Chimera::from_vec(vec!["Gamma", "Delta", "Void"]);
assert!(fast_vec.contains(&"Gamma"));

