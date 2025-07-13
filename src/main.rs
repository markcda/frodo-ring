#![no_std]

use frodo_ring::FrodoRing;

fn main() {
    let mut ring = FrodoRing::<u8, 6>::new();

    assert!(ring.push(0x1).is_ok());
    assert!(ring.push(0x2).is_ok());
    assert!(ring.push(0x3).is_ok());
    assert!(ring.push(0x4).is_ok());

    assert_eq!(ring.remove_at(1), Some(0x2));
    assert_eq!(ring.at(0), Some(&0x1));
    assert_eq!(ring.at(1), None);
    assert_eq!(ring.at(2), Some(&0x3));
    assert_eq!(ring.at(3), Some(&0x4));

    assert!(ring.push(0x5).is_ok());
    assert_eq!(ring.at(0), Some(&0x1));
    assert_eq!(ring.at(1), None);
    assert_eq!(ring.at(2), Some(&0x3));
    assert_eq!(ring.at(3), Some(&0x4));
    assert_eq!(ring.at(4), Some(&0x5));

    assert_eq!(ring.remove_at(0), Some(0x1));
    assert_eq!(ring.at(0), Some(&0x3));
    assert_eq!(ring.at(1), Some(&0x4));
    assert_eq!(ring.at(2), Some(&0x5));
    assert_eq!(ring.at(3), None);

    assert_eq!(ring.remove_at(1), Some(0x4));
    assert_eq!(ring.at(0), Some(&0x3));
    assert_eq!(ring.at(1), None);
    assert_eq!(ring.at(2), Some(&0x5));
    assert_eq!(ring.at(3), None);

    assert!(ring.push(0x6).is_ok());
    assert_eq!(ring.at(0), Some(&0x3));
    assert_eq!(ring.at(1), None);
    assert_eq!(ring.at(2), Some(&0x5));
    assert_eq!(ring.at(3), Some(&0x6));

    assert!(ring.push(0x7).is_ok());
    assert_eq!(ring.at(0), Some(&0x3));
    assert_eq!(ring.at(1), None);
    assert_eq!(ring.at(2), Some(&0x5));
    assert_eq!(ring.at(3), Some(&0x6));
    assert_eq!(ring.at(4), Some(&0x7));

    assert!(ring.push(0x8).is_ok());
}
