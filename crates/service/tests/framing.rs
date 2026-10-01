//! Framing is deterministic and closed before application parsing.
use cinder_service::transport::*;
use std::io::Cursor;
#[test]
fn lengths_truncation_zero_and_limits_reject() {
    for b in [
        &[0, 0, 0, 0][..],
        &[0, 0, 0, 33][..],
        &[0, 0, 0, 2, 1][..],
        &[0, 0][..],
    ] {
        assert!(read_frame(&mut Cursor::new(b), 32).is_err());
    }
    let mut wire = Vec::new();
    write_frame(&mut wire, &[8; 32], 32).unwrap();
    assert_eq!(&*read_frame(&mut Cursor::new(wire), 32).unwrap(), &[8; 32]);
    assert!(write_frame(&mut Vec::new(), &[], 32).is_err());
    assert!(write_frame(&mut Vec::new(), &[8; 33], 32).is_err());
}
