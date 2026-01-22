pub fn arr_insert<T: Copy>(arr: &mut [T], idx: usize, item: T) {
    for i in ((idx + 1)..arr.len()).rev() {
        arr[i] = arr[i - 1];
    }
    arr[idx] = item;
}

pub fn arr_remove<T: Copy>(arr: &mut [T], idx: usize) -> T {
    let item = arr[idx];
    for i in (idx + 1)..(arr.len() - 2) {
        arr[i] = arr[i + 1];
    }
    item
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_arr_insert() {
        let mut arr = [1, 2, 3, 4];
        arr_insert(&mut arr, 0, 0);
        assert_eq!(arr, [0, 1, 2, 3]);
        arr_insert(&mut arr, 3, 6);
        assert_eq!(arr, [0, 1, 2, 6]);
        arr_insert(&mut arr, 2, 5);
        assert_eq!(arr, [0, 1, 5, 2]);
    }

    fn test_arr_remove() {
        let mut arr = [1, 2, 3, 4, 0];
        arr_remove(&mut arr, 0);
        assert_eq!(arr, [2, 3, 4, 0, 0]);
        arr_remove(&mut arr, 3);
        assert_eq!(arr, [2, 3, 4, 0, 0]);
        arr_remove(&mut arr, 2);
        assert_eq!(arr, [2, 3, 0, 0, 0]);
    }
}
