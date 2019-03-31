use chips::Chip;
use jedec::Jedec;

#[no_mangle]
pub extern "C" fn set_and_c(
    jedec: *mut ::jedec::Jedec,
    row: u32,
    pin_num: u32,
    negation: u32,
) -> i32 {
    let mut jedec = unsafe { jedec.as_mut().unwrap() };
    jedec.check_magic();

    match set_and(
        &mut jedec,
        row as usize,
        pin_num as usize,
        negation != 0,
    ) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}

// Add an 'and' term to a fuse map.
fn set_and(
    jedec: &mut Jedec,
    row: usize,
    pin_num: usize,
    negation: bool,
) -> Result<(), String>{
    let chip = jedec.chip;
    let row_len = chip.num_cols();
    let column = jedec.pin_to_column(pin_num)?;

    // Is it a registered OLMC pin?
    // If yes, then correct the negation.
    let mut neg_off = if negation { 1 } else { 0 };
    if chip == Chip::GAL22V10 && (pin_num >= 14 && pin_num <= 23) && !jedec.s1[23 - pin_num] {
        neg_off = 1 - neg_off;
    }

    jedec.fuses[row * row_len + column + neg_off] = false;
    Ok(())
}
