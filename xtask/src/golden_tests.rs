use super::{changed_pixel_count, difference_report, safe_relative_output};

#[test]
fn one_changed_bit_is_one_changed_pixel() {
    assert_eq!(changed_pixel_count(&[0xff, 0xff], &[0x7f, 0xff]), 1);
    assert_eq!(changed_pixel_count(&[0x00], &[0xff]), 8);
}

#[test]
fn report_identifies_the_exact_changed_coordinate() {
    let expected = [0xff; 5_000];
    let mut actual = expected;
    actual[100 * 25 + 12] ^= 0x08;

    let report = difference_report(&expected, &actual);
    assert!(report.contains("changed_pixels=1"));
    assert!(report.contains("changed_coordinates=(100,100)"));
}

#[test]
fn destructive_outputs_are_limited_to_repository_children() {
    assert!(safe_relative_output("target/visual-diff").is_ok());
    assert!(safe_relative_output("/tmp/visual-diff").is_err());
    assert!(safe_relative_output("../visual-diff").is_err());
    assert!(safe_relative_output(".").is_err());
}
