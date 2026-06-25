use custom_kvm::{EdgeDetector, EdgeDetectionResult};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transition_to_mac() {
        let mut detector = EdgeDetector::new(1920, 1080);
        
        // Move virtual_x to 1918
        let current_x = detector.virtual_x();
        let dx = 1918 - current_x;
        assert_eq!(detector.update(dx, 0), EdgeDetectionResult::None);
        assert_eq!(detector.virtual_x(), 1918);

        // Move virtual_x by +1
        assert_eq!(detector.update(1, 0), EdgeDetectionResult::TransitionToMac);
        assert!(detector.is_controlling_mac());
    }

    #[test]
    fn test_transition_to_linux() {
        let mut detector = EdgeDetector::new(1920, 1080);
        
        // Transition to Mac
        detector.update(5000, 0);
        assert!(detector.is_controlling_mac());

        // Move virtual_x until it is 0
        let dx = -detector.virtual_x();
        assert_eq!(detector.update(dx, 0), EdgeDetectionResult::None);
        assert_eq!(detector.virtual_x(), 0);

        // Move virtual_x by -1
        assert_eq!(detector.update(-1, 0), EdgeDetectionResult::TransitionToLinux);
        assert!(!detector.is_controlling_mac());
        assert_eq!(detector.virtual_x(), 1910);
    }

    #[test]
    fn test_x_clamping_linux() {
        let mut detector = EdgeDetector::new(1920, 1080);
        
        // Move virtual_x by -10000
        detector.update(-10000, 0);
        assert_eq!(detector.virtual_x(), 0);

        // Move virtual_x by +5
        assert_eq!(detector.update(5, 0), EdgeDetectionResult::None);
        assert_eq!(detector.virtual_x(), 5);
    }

    #[test]
    fn test_y_clamping() {
        let mut detector = EdgeDetector::new(1920, 1080);
        
        // Move virtual_y by -10000
        detector.update(0, -10000);
        assert_eq!(detector.virtual_y(), 0);

        // Move virtual_y by +10000
        detector.update(0, 10000);
        assert_eq!(detector.virtual_y(), 1080);
    }

    #[test]
    fn test_large_dx_jump() {
        let mut detector = EdgeDetector::new(1920, 1080);
        
        // Perform a single update with dx = 5000
        assert_eq!(detector.update(5000, 0), EdgeDetectionResult::TransitionToMac);
    }

    #[test]
    fn test_reentry_buffer() {
        let mut detector = EdgeDetector::new(1920, 1080);
        
        // Transition to Mac
        detector.update(5000, 0);
        
        // Transition back to Linux
        detector.update(-10000, 0);
        
        assert_eq!(detector.virtual_x(), 1910);
    }
}
