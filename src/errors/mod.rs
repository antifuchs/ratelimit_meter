use std::time::Duration;

error_chain! {
    errors {
        /// Returned if the rate limiter implementation requires a
        /// capacity for the "bucket".
        CapacityRequired {
            display("a capacity is required")
        }

        /// Returned if the rate limiter implementation requires a
        /// weight per unit of work.
        WeightRequired {
            display("a weight is required")
        }

        /// Returned if the drainage time unit is wrong (e.g. it's negative).
        InvalidTimeUnit(u: Duration) {
            display("time unit {:?} is invalid", u)
        }
    }
}
