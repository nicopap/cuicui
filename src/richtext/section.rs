// TODO(feat): a vec<hashmap<x, t>> where item is either the actual value or
// index in vec of where to find x with given value.

enum OpRef<T> {
    Actual(T),
    Ref(usize),
}
