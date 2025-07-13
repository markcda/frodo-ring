# frodo-ring

Предоставляет реализацию очереди FIFO на кольцевом буфере, не использующем аллокации.

Пример:

```rust
fn main() -> Result<(), u8> {
    let mut ring = FrodoRing::<u8, 4>::new();

    ring.push(0x1)?;
    ring.push(0x2)?;
    ring.push(0x3)?;
    ring.push(0x4)?;

    println!("Первая ячейка: {}", ring.at(0).unwrap());
    println!("Последняя ячейка: {}", ring.at(-1).unwrap());

    ring.remove_at(1);

    assert_eq!(ring.at(0), Some(&0x1));
    assert_eq!(ring.at(1), None);
    assert_eq!(ring.at(2), Some(&0x3));
    assert_eq!(ring.at(3), ring.at(-1));

    assert_eq!(ring.get(0), Some(&0x1));
    assert_eq!(ring.get(1), Some(&0x3));
    assert_eq!(ring.get(2), Some(&0x4));

    let pos = ring.position(|el| *el == 0x3).unwrap();
    assert_eq!(ring.remove_at(pos), Some(0x3));

    println!("Элементы:");
    for el in ring.iter() {
        println!("\t{el}");
    }

    assert_eq!(ring.used(), 4);
    assert_eq!(ring.len(), 2);

    assert_eq!(ring.pick(), Some(0x1));

    assert_eq!(ring.used(), 1);
    assert_eq!(ring.len(), 1);

    Ok(())
}
```
