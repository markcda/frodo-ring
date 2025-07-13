//! Предоставляет реализацию очереди FIFO на кольцевом буфере, не использующем аллокации.

use core::mem::MaybeUninit;

/// Кольцевая очередь с порядком FIFO и не использующая аллокации.
///
/// У данной кольцевой очереди следующие ключевые особенности:
///
/// - два API: привычный (`get`/`len`/`iter`/`remove` с небольшим оверхедом `O(n)` на возможный поиск) и местный (`at`/`used`/`remove_at`)
/// - элементы могут быть изъяты из середины очереди без перемещения объектов в памяти, пока не достигнута максимальная ёмкость очереди
/// - смысл очереди - иметь возможность найти элемент с нужными предикатами, отсортированный в порядке очереди, в `no_std`-окружении.
pub struct FrodoRing<T, const N: usize> {
    /// Используется `MaybeUninit`, чтобы избежать инициализации и `Option`.
    buffer: [MaybeUninit<T>; N],
    /// При использовании отдельного массива `occupied` вместо `Option` мы можем рассчитывать на меньшую раскладку памяти.
    occupied: [bool; N],
    /// Указатель на начало очереди.
    head: usize,
    /// Используемая ёмкость очереди.
    ///
    /// В очереди всегда будут элементы `self.get(0)` и `self.get(self.used() - 1)`, если cap > 0.
    cap: usize,
}

impl<T: std::fmt::Debug, const N: usize> std::fmt::Debug for FrodoRing<T, N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "Ring: occupied = {}, head = {}, capacity = {}",
            self.occupied.iter().filter(|v| **v).count(),
            self.head,
            self.cap
        )?;
        writeln!(f, "Elements: [")?;
        for i in 0..N {
            if self.occupied[i] {
                writeln!(f, "\t{:?},", unsafe { self.buffer[i].assume_init_ref() })?;
            } else {
                writeln!(f, "\tNone,")?;
            }
        }
        writeln!(f, "]")?;

        Ok(())
    }
}

impl<T, const N: usize> Default for FrodoRing<T, N> {
    fn default() -> Self {
        Self {
            buffer: unsafe { MaybeUninit::uninit().assume_init() },
            occupied: [false; N],
            head: 0,
            cap: 0,
        }
    }
}

impl<T, const N: usize> FrodoRing<T, N> {
    /// Возвращает позицию N-ного элемента в кольце.
    fn real_pos(&self, naive_pos: usize) -> usize {
        (self.head + naive_pos) % N
    }

    /// Можно также передавать позицию с конца; например, `1` - это последний элемент.
    fn neg_pos(&self, naive_pos: usize) -> usize {
        (self.head + N - naive_pos) % N
    }

    /// Создаёт новую кольцевую очередь.
    pub fn new() -> Self {
        Self::default()
    }

    /// Возвращает использованное число ячеек кольцевой очереди.
    pub fn used(&self) -> usize {
        self.cap
    }

    /// Возвращает число элементов, находящихся в очереди.
    pub fn len(&self) -> usize {
        self.occupied.iter().filter(|v| **v).count()
    }

    /// Сообщает, есть ли в очереди элементы.
    pub fn is_empty(&self) -> bool {
        self.cap == 0
    }

    /// Получает элемент по ячейке (наивной позиции).
    ///
    /// Примеры:
    ///
    /// - `ring.at(0)` - получить первый элемент в очереди
    /// - `ring.at(1)` - получить содержимое ячейки следом за первым элементом (ячейка может быть пустой)
    /// - `ring.at(ring.used() - 1)` - получить последний элемент в очереди
    /// - `ring.at(-1)` - также получить последний элемент в очереди
    pub fn at(&self, naive_pos: isize) -> Option<&T> {
        if self.cap == 0 || naive_pos >= self.cap as isize || naive_pos < -(self.cap as isize) {
            return None;
        }

        let real_pos = if naive_pos >= 0 {
            self.real_pos(naive_pos as usize)
        } else {
            self.neg_pos((-naive_pos) as usize)
        };

        if self.occupied[real_pos] {
            Some(unsafe { self.buffer[real_pos].assume_init_ref() })
        } else {
            None
        }
    }

    /// Получает элемент по очереди.
    ///
    /// Примеры:
    ///
    /// - `ring.get(0)` - получить первый элемент в очереди
    /// - `ring.get(1)` - получить второй элемент в очереди
    /// - `ring.get(ring.len() - 1)` - получить последний элемент в очереди
    pub fn get(&self, pos: usize) -> Option<&T> {
        if pos >= self.cap || self.cap == 0 {
            return None;
        }

        let mut cntr = 0usize;
        let mut real_pos = self.head;
        let max_cntr = self.len();

        while cntr < max_cntr {
            if self.occupied[real_pos] {
                if cntr == pos {
                    return Some(unsafe { self.buffer[real_pos].assume_init_ref() });
                } else {
                    cntr += 1;
                }
            }
            real_pos = (real_pos + 1) % N;
        }

        None
    }

    /// Создаёт итератор по очереди.
    pub fn iter(&self) -> FrodoRingIterator<'_, T, N> {
        FrodoRingIterator {
            ring: self,
            naive_pos: 0,
        }
    }

    /// Получает наивную позицию (ячейку) элемента, отвечающего условию.
    ///
    /// Чтобы получить сам элемент, используйте `ring.at(naive_pos)`.
    pub fn position<F: Fn(&T) -> bool>(&self, f: F) -> Option<isize> {
        let mut real_pos = self.head;
        let last_pos = self.neg_pos(1);

        while real_pos <= last_pos {
            if self.occupied[real_pos] && f(unsafe { self.buffer[real_pos].assume_init_ref() }) {
                return Some(real_pos as isize);
            }
            real_pos = (real_pos + 1) % N;
        }

        None
    }

    /// Кладёт элемент в очередь.
    ///
    /// В случае, если число использованных очередью ячеек равно N, но при этом хотя бы одна из них не занята,
    /// очередь проводит операцию сжатия (`O(n)`) с перемещением элементов в памяти.
    pub fn push(&mut self, item: T) -> Result<(), T> {
        let real_pos = if self.cap == N {
            if self.occupied.iter().all(|o| *o) {
                return Err(item);
            } else if let Some(tail) = self.compact() {
                tail
            } else {
                return Err(item);
            }
        } else {
            self.real_pos(self.cap)
        };

        self.buffer[real_pos].write(item);
        self.occupied[real_pos] = true;
        self.cap += 1;
        Ok(())
    }

    /// Отдаёт первый элемент, изымая его из очереди.
    pub fn pick(&mut self) -> Option<T> {
        self.remove_at(0)
    }

    /// Удаляет содержимое ячейки, находящейся по наивной позиции, и возвращает его.
    pub fn remove_at(&mut self, naive_pos: isize) -> Option<T> {
        if self.cap == 0 || naive_pos >= self.cap as isize || naive_pos < -(self.cap as isize) {
            return None;
        }

        let real_pos = if naive_pos >= 0 {
            self.real_pos(naive_pos as usize)
        } else {
            self.neg_pos((-naive_pos) as usize)
        };

        if self.occupied[real_pos] {
            self.occupied[real_pos] = false;

            if real_pos == self.head {
                loop {
                    self.head = (self.head + 1) % N;
                    self.cap -= 1;
                    if self.occupied[self.head] || self.cap == 0 {
                        break;
                    }
                }
            } else if real_pos == self.neg_pos(1) {
                loop {
                    if self.occupied[self.real_pos(self.cap - 1)] || self.cap == 1 {
                        break;
                    }
                    self.cap -= 1;
                }
            }

            Some(unsafe { self.buffer[real_pos].assume_init_read() })
        } else {
            None
        }
    }

    /// Удаляет элемент из очереди.
    pub fn remove(&mut self, pos: usize) -> Option<T> {
        if pos >= self.cap || self.cap == 0 {
            return None;
        }

        let mut cntr = 0usize;
        let mut real_pos = self.head;
        let max_cntr = self.len();

        while cntr < max_cntr {
            if self.occupied[real_pos] {
                if cntr == pos {
                    self.occupied[real_pos] = false;

                    if real_pos == self.head {
                        loop {
                            self.head = (self.head + 1) % N;
                            self.cap -= 1;
                            if self.occupied[self.head] || self.cap == 0 {
                                break;
                            }
                        }
                    } else if real_pos == self.neg_pos(1) {
                        loop {
                            if self.occupied[self.real_pos(self.cap - 1)] || self.cap == 1 {
                                break;
                            }
                            self.cap -= 1;
                        }
                    }

                    return Some(unsafe { self.buffer[real_pos].assume_init_read() });
                } else {
                    cntr += 1;
                }
            }
            real_pos = (real_pos + 1) % N;
        }

        None
    }

    /// Ужимает место в буфере, сохраняя порядок расположения элементов.
    ///
    /// Возвращает последнее пустое место (real_pos), куда можно вставить элемент.
    ///
    /// Важно: метод опирается на то, что первый элемент никогда не будет пустым (`self.real_pos(self.head)`).
    fn compact(&mut self) -> Option<usize> {
        assert_eq!(self.cap, N);

        let mut read_pos = 0usize;
        let mut read_real_pos = self.real_pos(read_pos);

        let mut write_pos = 0usize;
        let mut write_real_pos = self.real_pos(write_pos);
        let mut moved = 0usize;

        let last_pos = self.cap - 1;

        while read_pos <= last_pos {
            // Пока элементы совпадают, идём и ищем пропуски
            if read_pos == write_pos && self.occupied[read_real_pos] {
                read_pos += 1;
                read_real_pos = self.real_pos(read_pos);
                write_pos = read_pos;
                write_real_pos = read_real_pos;
                continue;
            }

            // Если находим пустую ячейку, - перемещаем туда указатель на запись
            if !self.occupied[read_real_pos] {
                read_pos += 1;
                read_real_pos = self.real_pos(read_pos);
                moved += 1;
            } else {
                self.occupied[read_real_pos] = false;
                self.occupied[write_real_pos] = true;
                let item = unsafe { self.buffer[read_real_pos].assume_init_read() };
                self.buffer[write_real_pos].write(item);

                read_pos += 1;
                read_real_pos = self.real_pos(read_pos);
                write_pos += 1;
                write_real_pos = self.real_pos(write_pos);
            }
        }

        if moved > 0 {
            self.cap -= moved;
            Some(self.real_pos(self.cap))
        } else {
            None
        }
    }
}

/// Итератор по элементам очереди.
///
/// При итерировании пропускает пустые ячейки, выдавая исключительно присутствующие элементы.
pub struct FrodoRingIterator<'ring, T, const N: usize> {
    ring: &'ring FrodoRing<T, N>,
    naive_pos: usize,
}

impl<'ring, T: std::fmt::Debug, const N: usize> Iterator for FrodoRingIterator<'ring, T, N> {
    type Item = &'ring T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.naive_pos == self.ring.cap {
                return None;
            }
            let res = self.ring.at(self.naive_pos as isize);
            self.naive_pos += 1;
            if res.is_some() {
                return res;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_1() {
        let mut ring = FrodoRing::<u8, 4>::new();

        assert!(ring.push(0x1).is_ok());
        assert!(ring.push(0x2).is_ok());
        assert!(ring.push(0x3).is_ok());
        assert!(ring.push(0x4).is_ok());

        assert!(ring.push(0x5).is_err());
    }

    #[test]
    fn test_2() {
        let mut ring = FrodoRing::<u8, 4>::new();

        assert!(ring.push(0x1).is_ok());
        assert!(ring.push(0x2).is_ok());
        assert!(ring.push(0x3).is_ok());
        assert!(ring.push(0x4).is_ok());

        assert_eq!(ring.at(0), Some(&0x1));
        assert_eq!(ring.at(1), Some(&0x2));
        assert_eq!(ring.at(2), Some(&0x3));
        assert_eq!(ring.at(3), Some(&0x4));
        assert_eq!(ring.at(-1), Some(&0x4));
        assert_eq!(ring.at(-2), Some(&0x3));
        assert_eq!(ring.at(-3), Some(&0x2));
        assert_eq!(ring.at(-4), Some(&0x1));

        assert_eq!(ring.at(4), None);
        assert_eq!(ring.at(-5), None);
    }

    #[test]
    fn test_3() {
        let mut ring = FrodoRing::<u8, 4>::new();

        assert!(ring.push(0x1).is_ok());
        assert!(ring.push(0x2).is_ok());
        assert!(ring.push(0x3).is_ok());
        assert!(ring.push(0x4).is_ok());

        assert_eq!(ring.remove_at(1), Some(0x2));
        assert_eq!(ring.at(0), Some(&0x1));
        assert_eq!(ring.at(1), None);
        assert_eq!(ring.at(2), Some(&0x3));
        assert_eq!(ring.at(3), Some(&0x4));
    }

    #[test]
    fn test_4() {
        let mut ring = FrodoRing::<u8, 4>::new();

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
        assert_eq!(ring.at(1), Some(&0x3));
        assert_eq!(ring.at(2), Some(&0x4));
        assert_eq!(ring.at(3), Some(&0x5));
    }

    #[test]
    fn massive() {
        let mut ring = FrodoRing::<u8, 4>::new();

        assert!(ring.push(0x1).is_ok());
        assert!(ring.push(0x2).is_ok());
        assert!(ring.push(0x3).is_ok());
        assert!(ring.push(0x4).is_ok());

        assert_eq!(ring.remove_at(1), Some(0x2));
        assert_eq!(ring.used(), 4);
        assert_eq!(ring.at(0), Some(&0x1));
        assert_eq!(ring.at(1), None);
        assert_eq!(ring.at(2), Some(&0x3));
        assert_eq!(ring.at(3), Some(&0x4));

        assert!(ring.push(0x5).is_ok());
        assert_eq!(ring.used(), 4);
        assert_eq!(ring.at(0), Some(&0x1));
        assert_eq!(ring.at(1), Some(&0x3));
        assert_eq!(ring.at(2), Some(&0x4));
        assert_eq!(ring.at(3), Some(&0x5));

        assert_eq!(ring.remove_at(0), Some(0x1));
        assert_eq!(ring.used(), 3);
        assert_eq!(ring.at(0), Some(&0x3));
        assert_eq!(ring.at(1), Some(&0x4));
        assert_eq!(ring.at(2), Some(&0x5));
        assert_eq!(ring.at(3), None);

        assert_eq!(ring.remove_at(1), Some(0x4));
        assert_eq!(ring.used(), 3);
        assert_eq!(ring.at(0), Some(&0x3));
        assert_eq!(ring.at(1), None);
        assert_eq!(ring.at(2), Some(&0x5));
        assert_eq!(ring.at(3), None);

        assert!(ring.push(0x6).is_ok());
        assert_eq!(ring.used(), 4);
        assert_eq!(ring.at(0), Some(&0x3));
        assert_eq!(ring.at(1), None);
        assert_eq!(ring.at(2), Some(&0x5));
        assert_eq!(ring.at(3), Some(&0x6));

        assert!(ring.push(0x7).is_ok());
        assert_eq!(ring.used(), 4);
        assert_eq!(ring.at(0), Some(&0x3));
        assert_eq!(ring.at(1), Some(&0x5));
        assert_eq!(ring.at(2), Some(&0x6));
        assert_eq!(ring.at(3), Some(&0x7));

        assert!(ring.push(0x8).is_err());
    }

    #[test]
    fn iter() {
        let mut ring = FrodoRing::<u8, 4>::new();

        assert!(ring.push(0x1).is_ok());
        assert!(ring.push(0x2).is_ok());
        assert!(ring.push(0x3).is_ok());
        assert!(ring.push(0x4).is_ok());

        assert_eq!(ring.remove_at(1), Some(0x2));
        let mut it = ring.iter();
        assert_eq!(it.next(), Some(&0x1));
        assert_eq!(it.next(), Some(&0x3));
        assert_eq!(it.next(), Some(&0x4));
        assert_eq!(it.next(), None);

        assert!(ring.push(0x5).is_ok());
        let mut it = ring.iter();
        assert_eq!(it.next(), Some(&0x1));
        assert_eq!(it.next(), Some(&0x3));
        assert_eq!(it.next(), Some(&0x4));
        assert_eq!(it.next(), Some(&0x5));
        assert_eq!(it.next(), None);

        assert_eq!(ring.remove_at(0), Some(0x1));
        let mut it = ring.iter();
        assert_eq!(it.next(), Some(&0x3));
        assert_eq!(it.next(), Some(&0x4));
        assert_eq!(it.next(), Some(&0x5));
        assert_eq!(it.next(), None);

        assert_eq!(ring.remove_at(1), Some(0x4));
        let mut it = ring.iter();
        assert_eq!(it.next(), Some(&0x3));
        assert_eq!(it.next(), Some(&0x5));
        assert_eq!(ring.at(3), None);

        assert!(ring.push(0x6).is_ok());
        let mut it = ring.iter();
        assert_eq!(it.next(), Some(&0x3));
        assert_eq!(it.next(), Some(&0x5));
        assert_eq!(it.next(), Some(&0x6));
        assert_eq!(it.next(), None);
        assert_eq!(it.next(), None);
        assert_eq!(it.next(), None);

        assert!(ring.push(0x7).is_ok());
        let mut it = ring.iter();
        assert_eq!(it.next(), Some(&0x3));
        assert_eq!(it.next(), Some(&0x5));
        assert_eq!(it.next(), Some(&0x6));
        assert_eq!(it.next(), Some(&0x7));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn test_5() {
        let mut ring = FrodoRing::<u8, 4>::new();

        assert!(ring.push(0x1).is_ok());
        assert!(ring.push(0x2).is_ok());
        assert!(ring.push(0x3).is_ok());
        assert!(ring.push(0x4).is_ok());

        assert_eq!(ring.remove_at(1), Some(0x2));
        assert_eq!(ring.used(), 4);
        assert_eq!(ring.at(0), Some(&0x1));
        assert_eq!(ring.at(1), None);
        assert_eq!(ring.at(2), Some(&0x3));
        assert_eq!(ring.at(3), Some(&0x4));

        assert_eq!(ring.remove_at(2), Some(0x3));
        assert_eq!(ring.used(), 4);
        assert_eq!(ring.at(0), Some(&0x1));
        assert_eq!(ring.at(1), None);
        assert_eq!(ring.at(2), None);
        assert_eq!(ring.at(3), Some(&0x4));

        assert_eq!(ring.remove_at(0), Some(0x1));
        assert_eq!(ring.used(), 1);
        assert_eq!(ring.at(0), Some(&0x4));
        assert_eq!(ring.at(1), None);
        assert_eq!(ring.at(2), None);
        assert_eq!(ring.at(3), None);
    }

    #[test]
    fn test_6() {
        let mut ring = FrodoRing::<u8, 4>::new();

        assert!(ring.push(0x1).is_ok());
        assert!(ring.push(0x2).is_ok());
        assert!(ring.push(0x3).is_ok());
        assert!(ring.push(0x4).is_ok());

        assert_eq!(ring.remove_at(1), Some(0x2));
        assert_eq!(ring.used(), 4);
        assert_eq!(ring.at(0), Some(&0x1));
        assert_eq!(ring.at(1), None);
        assert_eq!(ring.at(2), Some(&0x3));
        assert_eq!(ring.at(3), Some(&0x4));

        assert_eq!(ring.remove_at(2), Some(0x3));
        assert_eq!(ring.used(), 4);
        assert_eq!(ring.at(0), Some(&0x1));
        assert_eq!(ring.at(1), None);
        assert_eq!(ring.at(2), None);
        assert_eq!(ring.at(3), Some(&0x4));

        assert_eq!(ring.remove_at(3), Some(0x4));
        assert_eq!(ring.used(), 1);
        assert_eq!(ring.at(0), Some(&0x1));
        assert_eq!(ring.at(1), None);
        assert_eq!(ring.at(2), None);
        assert_eq!(ring.at(3), None);
    }

    #[test]
    fn test_7() {
        let mut ring = FrodoRing::<u8, 4>::new();

        assert!(ring.push(0x1).is_ok());
        assert!(ring.push(0x2).is_ok());
        assert!(ring.push(0x3).is_ok());
        assert!(ring.push(0x4).is_ok());

        assert_eq!(ring.pick(), Some(0x1));
        assert_eq!(ring.pick(), Some(0x2));
        assert_eq!(ring.pick(), Some(0x3));
        assert_eq!(ring.pick(), Some(0x4));
        assert_eq!(ring.pick(), None);
    }

    #[test]
    fn test_8() {
        let mut ring = FrodoRing::<u8, 4>::new();

        assert!(ring.push(0x1).is_ok());
        assert!(ring.push(0x2).is_ok());
        assert!(ring.push(0x3).is_ok());
        assert!(ring.push(0x4).is_ok());

        assert_eq!(ring.at(0), Some(&0x1));
        assert_eq!(ring.at(1), Some(&0x2));
        assert_eq!(ring.at(2), Some(&0x3));
        assert_eq!(ring.at(3), Some(&0x4));
        assert_eq!(ring.get(0), Some(&0x1));
        assert_eq!(ring.get(1), Some(&0x2));
        assert_eq!(ring.get(2), Some(&0x3));
        assert_eq!(ring.get(3), Some(&0x4));

        assert_eq!(ring.get(4), None);

        assert_eq!(ring.remove_at(1), Some(0x2));
        assert_eq!(ring.used(), 4);
        assert_eq!(ring.at(0), Some(&0x1));
        assert_eq!(ring.at(1), None);
        assert_eq!(ring.at(2), Some(&0x3));
        assert_eq!(ring.at(3), Some(&0x4));
        assert_eq!(ring.get(0), Some(&0x1));
        assert_eq!(ring.get(1), Some(&0x3));
        assert_eq!(ring.get(2), Some(&0x4));
        assert_eq!(ring.get(3), None);
    }

    #[test]
    fn test_9() {
        let mut ring = FrodoRing::<u8, 4>::new();

        assert!(ring.push(0x1).is_ok());
        assert!(ring.push(0x2).is_ok());
        assert!(ring.push(0x3).is_ok());
        assert!(ring.push(0x4).is_ok());

        assert_eq!(ring.remove(1), Some(0x2));
        assert_eq!(ring.used(), 4);
        assert_eq!(ring.at(0), Some(&0x1));
        assert_eq!(ring.at(1), None);
        assert_eq!(ring.at(2), Some(&0x3));
        assert_eq!(ring.at(3), Some(&0x4));

        assert_eq!(ring.remove(1), Some(0x3));
        assert_eq!(ring.used(), 4);
        assert_eq!(ring.at(0), Some(&0x1));
        assert_eq!(ring.at(1), None);
        assert_eq!(ring.at(2), None);
        assert_eq!(ring.at(3), Some(&0x4));

        assert_eq!(ring.remove(1), Some(0x4));
        assert_eq!(ring.used(), 1);
        assert_eq!(ring.at(0), Some(&0x1));
        assert_eq!(ring.at(1), None);
        assert_eq!(ring.at(2), None);
        assert_eq!(ring.at(3), None);
    }
}
