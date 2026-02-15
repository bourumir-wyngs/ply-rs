use ply_rs_bw::ply::GetProperty;

struct Wide {
    x_i64: i64,
    x_u64: u64,
    x_i128: i128,
    x_u128: u128,
}

#[test]
fn wide_integer_scalars_are_exposed_via_int_uint_getters_when_in_range() {
    let w = Wide {
        x_i64: -123,
        x_u64: 456,
        x_i128: 789,
        x_u128: 1011,
    };

    assert_eq!(GetProperty::<i32>::get(&w.x_i64), Some(-123));
    assert_eq!(GetProperty::<u32>::get(&w.x_u64), Some(456));
    assert_eq!(GetProperty::<i32>::get(&w.x_i128), Some(789));
    assert_eq!(GetProperty::<u32>::get(&w.x_u128), Some(1011));
}

#[test]
fn wide_integer_getters_return_none_on_overflow() {
    let w = Wide {
        x_i64: i64::from(i32::MAX) + 1,
        x_u64: u64::from(u32::MAX) + 1,
        x_i128: i128::from(i32::MIN) - 1,
        x_u128: u128::from(u32::MAX) + 1,
    };

    assert_eq!(GetProperty::<i32>::get(&w.x_i64), None);
    assert_eq!(GetProperty::<u32>::get(&w.x_u64), None);
    assert_eq!(GetProperty::<i32>::get(&w.x_i128), None);
    assert_eq!(GetProperty::<u32>::get(&w.x_u128), None);
}
