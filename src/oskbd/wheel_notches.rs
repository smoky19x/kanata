//! Convert kanata's wheel distance into whole wheel notches.
//!
//! Wheel distances use the unit documented for Linux and Windows:
//! `HI_RES_SCROLL_UNITS_IN_LO_RES` (120) units are one notch. Outputs that can
//! only express whole notches, such as the macOS virtual HID pointing device,
//! use this to convert and to carry what is left over to the next scroll.

use super::HI_RES_SCROLL_UNITS_IN_LO_RES;
use kanata_parser::custom_action::MWheelDirection;

const NOTCH: i32 = HI_RES_SCROLL_UNITS_IN_LO_RES as i32;

/// Distance below one notch, kept per axis until it adds up to a notch.
#[derive(Debug, Default)]
pub struct WheelNotches {
    /// Remainder of the vertical axis, positive = up.
    vertical: i32,
    /// Remainder of the horizontal axis, positive = right.
    horizontal: i32,
}

impl WheelNotches {
    /// Add a scroll and return the whole notches to send now on the axis of
    /// `direction`: positive = up or right, negative = down or left.
    ///
    /// A scroll in the opposite direction drops the remainder first, so a bit
    /// of distance up followed by a bit down never adds up to a notch.
    pub fn add(&mut self, direction: MWheelDirection, distance: u16) -> i32 {
        let distance = i32::from(distance);
        let (remainder, distance) = match direction {
            MWheelDirection::Up => (&mut self.vertical, distance),
            MWheelDirection::Down => (&mut self.vertical, -distance),
            MWheelDirection::Right => (&mut self.horizontal, distance),
            MWheelDirection::Left => (&mut self.horizontal, -distance),
        };
        if remainder.signum() == -distance.signum() {
            *remainder = 0;
        }
        *remainder += distance;
        let notches = *remainder / NOTCH;
        *remainder -= notches * NOTCH;
        notches
    }
}

/// Split a notch count into values for HID wheel reports, which hold
/// -127..=127 each.
pub fn hid_wheel_values(mut notches: i32) -> impl Iterator<Item = i8> {
    std::iter::from_fn(move || {
        if notches == 0 {
            return None;
        }
        let value = notches.clamp(-127, 127);
        notches -= value;
        Some(value as i8)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use MWheelDirection::*;

    #[test]
    fn one_notch_per_120_units() {
        let mut n = WheelNotches::default();
        assert_eq!(n.add(Up, 120), 1);
        assert_eq!(n.add(Down, 240), -2);
        assert_eq!(n.add(Right, 120), 1);
        assert_eq!(n.add(Left, 360), -3);
    }

    #[test]
    fn remainder_is_carried_per_axis() {
        let mut n = WheelNotches::default();
        assert_eq!(n.add(Down, 60), 0);
        assert_eq!(n.add(Right, 60), 0);
        assert_eq!(n.add(Down, 60), -1);
        assert_eq!(n.add(Down, 100), 0);
        assert_eq!(n.add(Down, 200), -2);
        assert_eq!(n.add(Right, 60), 1);
    }

    #[test]
    fn reversing_drops_the_remainder() {
        let mut n = WheelNotches::default();
        assert_eq!(n.add(Up, 60), 0);
        assert_eq!(n.add(Down, 60), 0);
        assert_eq!(n.add(Down, 60), -1);
        assert_eq!(n.add(Right, 100), 0);
        assert_eq!(n.add(Left, 100), 0);
        assert_eq!(n.add(Right, 100), 0);
        assert_eq!(n.add(Right, 20), 1);
    }

    #[test]
    fn zero_distance_sends_nothing() {
        let mut n = WheelNotches::default();
        assert_eq!(n.add(Up, 100), 0);
        assert_eq!(n.add(Down, 0), 0);
        assert_eq!(n.add(Up, 20), 1);
    }

    #[test]
    fn largest_distance_does_not_overflow() {
        let mut n = WheelNotches::default();
        assert_eq!(n.add(Down, u16::MAX), -546);
        assert_eq!(n.add(Down, 105), -1);
    }

    #[test]
    fn hid_values_are_split_into_report_range() {
        assert_eq!(hid_wheel_values(0).count(), 0);
        assert_eq!(hid_wheel_values(5).collect::<Vec<_>>(), [5]);
        assert_eq!(hid_wheel_values(-127).collect::<Vec<_>>(), [-127]);
        assert_eq!(hid_wheel_values(300).collect::<Vec<_>>(), [127, 127, 46]);
        assert_eq!(
            hid_wheel_values(-546).collect::<Vec<_>>(),
            [-127, -127, -127, -127, -38]
        );
    }
}
