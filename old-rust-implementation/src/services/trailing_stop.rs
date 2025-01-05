use crate::models::side::Side;

/// [TrailingStop] will track the current price and adjust the desired stop loss accordingly.
///
/// **How it works ?** you will configure how the trailing/following will trigger relative to the current price,
/// and the [TrailingStop] will signal when you should exit the position when calling the function [TrailingStop::should_close_position]
///
/// **important:** you must call the function [TrailingStop::should_close_position] with every new price update
/// to allow the trailing stop to properly **adjust** the desired **stop loss**
pub struct TrailingStop {
    entry_price: f64,
    stop_loss_price: f64,
    points_per_tick: f64,
    direction: Side,
    is_trailing_active: bool,
    is_finished: bool,
    activation_price: f64,
    price_next_trailing_move: f64,
    ticks_frequency_trailing_update: usize,
    distance_in_ticks_from_current_price: usize,
    max_ticks_in_positive_direction: usize,
}

impl TrailingStop {
    /// Create a new [TrailingStop] with the configuration provided
    ///
    /// **current_price:** is the latest price available for the asset being traded.
    /// **direction:** is the [Side] of the position you will open long/short.
    /// **points_per_tick:** reflects how many points a single price variation represents in the traded asset
    /// **ticks_stop_loss:** offset in ticks used to calculate the initial stop loss price relative to the current_price
    /// **ticks_trigger_start_trailing:** off set used to calculate the initial price when the trailing will be activated
    /// **ticks_first_trailing_trigger:** how many ticks after the price defined in by ticks_trigger_start_trailing that the stop
    /// price will be updated
    /// **ticks_distance_when_trailing_current_price:** distance in ticks from the current price for the trailing stop after
    /// each update
    /// **ticks_frequency_trailing_update:** frequency for trailing to be updated ( amount of positive ticks in the direction of your trade )
    pub fn new(
        current_price: f64,
        direction: Side,
        points_per_tick: f64,
        ticks_stop_loss: usize, // 8
        ticks_trigger_start_trailing: usize, // 13
        ticks_first_trailing_trigger: usize, // 5
        ticks_distance_when_trailing_current_price: usize, // 2
        ticks_frequency_trailing_update: usize, // 1
    ) -> Self {
        match direction {
            Side::BUY => {
                let trailing_activation_price = current_price + (points_per_tick * ticks_trigger_start_trailing as f64);
                TrailingStop {
                    entry_price: current_price,
                    stop_loss_price: current_price - points_per_tick * ticks_stop_loss as f64,
                    points_per_tick,
                    direction,
                    is_finished: false,
                    is_trailing_active: false,
                    activation_price: trailing_activation_price,
                    price_next_trailing_move: trailing_activation_price + (ticks_first_trailing_trigger as f64 * points_per_tick),
                    ticks_frequency_trailing_update,
                    distance_in_ticks_from_current_price: ticks_distance_when_trailing_current_price,
                    max_ticks_in_positive_direction: 0,
                }
            }
            Side::SELL => {
                let trailing_activation_price = current_price - (points_per_tick * ticks_trigger_start_trailing as f64);
                TrailingStop {
                    entry_price: current_price,
                    stop_loss_price: current_price + points_per_tick * ticks_stop_loss as f64,
                    points_per_tick,
                    direction,
                    is_finished: false,
                    is_trailing_active: false,
                    activation_price: trailing_activation_price,
                    price_next_trailing_move: trailing_activation_price - (ticks_first_trailing_trigger as f64 * points_per_tick),
                    ticks_frequency_trailing_update,
                    distance_in_ticks_from_current_price: ticks_distance_when_trailing_current_price,
                    max_ticks_in_positive_direction: 0,
                }
            }
        }
    }

    pub fn new_not_automatic_trailing(entry_price: f64, side: Side, points_per_tick: f64) -> Self {
        TrailingStop {
            entry_price,
            stop_loss_price: match side {
                Side::BUY => f64::MIN,
                Side::SELL => f64::MAX
            },
            points_per_tick,
            direction: side,
            is_finished: false,
            is_trailing_active: false,
            activation_price: match side {
                Side::BUY => f64::MAX,
                Side::SELL => f64::MIN
            },
            price_next_trailing_move: match side {
                Side::BUY => f64::MIN,
                Side::SELL => f64::MAX
            },
            ticks_frequency_trailing_update: 99999,
            distance_in_ticks_from_current_price: 999999,
            max_ticks_in_positive_direction: 0,
        }
    }

    #[allow(unused)]
    /// will ignore the previously defined activation_price and start trailing the current price
    pub fn start_trailing_now(&mut self) {
        self.is_trailing_active = true;
    }

    pub fn finish_trailing(&mut self){
        self.is_finished = true;
    }

    #[allow(unused)]
    pub fn is_result_positive(&self, current_price: f64) -> bool {
        match self.direction {
            Side::BUY => current_price >= self.entry_price,
            Side::SELL => current_price <= self.entry_price,
        }
    }

    #[allow(unused)]
    pub fn max_tick_in_positive_direction(&self) -> usize {
        self.max_ticks_in_positive_direction
    }

    pub fn set_stop_loss(&mut self, new_stop_loss_price: f64) {
        self.stop_loss_price = new_stop_loss_price;
    }

    #[allow(unused)]
    pub fn resulting_ticks_pnl(&self, current_price: f64) -> i64 {
        match self.direction {
            Side::BUY => ((current_price - self.entry_price) / self.points_per_tick)  as i64,
            Side::SELL => ((self.entry_price - current_price) / self.points_per_tick) as i64,
        }
    }


    /// check if the position should be closed now
    pub fn should_close_position(&mut self, current_price: f64) -> bool {
        match self.direction {
            Side::BUY => {
                if current_price > self.entry_price {
                    let diff = (current_price - self.entry_price) / self.points_per_tick;
                    if diff > self.max_ticks_in_positive_direction as f64 {
                        self.max_ticks_in_positive_direction= diff as usize;
                    }
                }

                if self.is_finished || current_price <= self.stop_loss_price {
                    self.is_finished = true;
                    return true;
                }

                // activate trailing stop
                if !self.is_trailing_active && current_price >= self.activation_price {
                    self.is_trailing_active = true;
                }

                // move trailing stop
                if self.is_trailing_active && current_price >= self.price_next_trailing_move {
                    self.stop_loss_price = current_price - (self.points_per_tick * self.distance_in_ticks_from_current_price as f64);
                    self.price_next_trailing_move += self.points_per_tick * self.ticks_frequency_trailing_update as f64;
                }
            }
            Side::SELL => {
                if current_price < self.entry_price {
                    let diff = (self.entry_price - current_price)  / self.points_per_tick;
                    if diff > self.max_ticks_in_positive_direction as f64 {
                        self.max_ticks_in_positive_direction = diff as usize;
                    }
                }

                if self.is_finished || current_price >= self.stop_loss_price {
                    self.is_finished = true;
                    return true;
                }

                // activate trailing stop
                if !self.is_trailing_active && current_price <= self.activation_price {
                    self.is_trailing_active = true;
                }

                // move trailing stop
                if self.is_trailing_active && current_price <= self.price_next_trailing_move {
                    self.stop_loss_price = current_price + (self.points_per_tick * self.distance_in_ticks_from_current_price as f64);
                    self.price_next_trailing_move -= self.points_per_tick * self.ticks_frequency_trailing_update as f64;
                }
            }
        }

        false
    }

    #[allow(unused)]
    pub fn is_trailing_active(&self) -> bool {
        self.is_trailing_active
    }

    pub fn is_finished(&self) -> bool {
        self.is_finished
    }
}

#[cfg(test)]
mod test {
    use crate::models::side::Side;
    use crate::services::trailing_stop::TrailingStop;

    #[test]
    fn should_create_trailing_stop_correctly() {
        let trailing_stop = TrailingStop::new(
            119_980.0,
            Side::BUY,
            5.0,
            8,
            13,
            5,
            2,
            1,
        );

        assert_eq!(trailing_stop.is_trailing_active, false);
        assert_eq!(trailing_stop.is_finished, false);
    }

    #[test]
    fn should_activate_trailing_stop_correctly() {
        let mut trailing_stop = TrailingStop::new(
            119_980.0,
            Side::BUY,
            5.0,
            8,
            13,
            5,
            2,
            1,
        );

        trailing_stop.start_trailing_now();

        assert_eq!(trailing_stop.is_trailing_active, true);
        assert_eq!(trailing_stop.is_finished, false);
    }

    #[test]
    fn should_set_trail_price_correctly() {
        let tests: Vec<(Side, f64, f64, usize, usize, usize, usize, usize, Vec<(f64, f64)>)> = vec![
            (Side::BUY, 5.0, 119_840.0, 8, 13, 5, 2, 1, vec![(119_930.0, 119_920.0), (119_950.0, 119_940.0)]),
            (Side::BUY, 5.0, 119_840.0, 8, 13, 5, 2, 1, vec![(119_930.0, 119_920.0), (119_935.0, 119_925.0), (119_940.0, 119_930.0)]),
            (Side::BUY, 5.0, 119_840.0, 8, 13, 5, 4, 1, vec![(119_930.0, 119_910.0), (119_950.0, 119_930.0)]),
            (Side::BUY, 5.0, 119_840.0, 8, 13, 5, 2, 2, vec![(119_930.0, 119_920.0), (119_935.0, 119_920.0),
                                                             (119_940.0, 119_930.0), (119_945.0, 119_930.0), (119_950.0, 119_940.0)]),

            (Side::SELL, 5.0, 119_840.0, 8, 13, 5, 2, 1, vec![(119_750.0, 119_760.0), (119_730.0, 119_740.0)]),
            (Side::SELL, 5.0, 119_840.0, 8, 13, 5, 2, 1, vec![(119_750.0, 119_760.0), (119_745.0, 119_755.0), (119_740.0, 119_750.0)]),
            (Side::SELL, 5.0, 119_840.0, 8, 13, 5, 4, 1, vec![(119_750.0, 119_770.0), (119_730.0, 119_750.0)]),
            (Side::SELL, 5.0, 119_840.0, 8, 13, 5, 2, 2, vec![(119_750.0, 119_760.0), (119_745.0, 119_760.0),
                                                             (119_740.0, 119_750.0), (119_735.0, 119_750.0), (119_730.0, 119_740.0)]),
        ];

        for test_values in tests {
            let side = test_values.0;
            let points_per_tick = test_values.1;
            let current_price = test_values.2;

            let ticks_stop_loss = test_values.3;
            let ticks_trigger_start_trailing = test_values.4;
            let ticks_first_trailing_trigger = test_values.5; // 5
            let ticks_distance_when_trailing_current_price = test_values.6;
            let ticks_frequency_trailing_update = test_values.7;

            let mut trailing_stop = TrailingStop::new(
                current_price,
                side,
                points_per_tick,
                ticks_stop_loss,
                ticks_trigger_start_trailing,
                ticks_first_trailing_trigger,
                ticks_distance_when_trailing_current_price,
                ticks_frequency_trailing_update,
            );

            let price_updates = test_values.8;

            for (current_price, expected_stop_loss) in price_updates {
                assert_eq!(false, trailing_stop.should_close_position(current_price),
                           "should not close position with current price of {current_price}");
                assert_eq!(expected_stop_loss, trailing_stop.stop_loss_price,
                           "{side} position stop loss should be updated to {expected_stop_loss} when current price is {current_price} \
                             but it was {} | trigger price to move stop is {}", trailing_stop.stop_loss_price, trailing_stop.activation_price);
            }
        }
    }

    #[test]
    fn should_set_as_finish_when_stop_loss_is_triggered() {
        // side, points_per_tick, current_price, ticks_stop_loss,
        // current_price_dont_trigger_trailing, current_price_trigger_stop_loss
        let tests: Vec<(Side, f64, f64, usize, f64, f64)> = vec![
            (Side::BUY, 5.0, 119_980.0, 8, 119_960.0, 119_940.0),
            (Side::BUY, 5.0, 119_980.0, 8, 119_965.0, 119_900.0),
            (Side::SELL, 5.0, 119_920.0, 16, 119_900.0, 120_060.0),
            (Side::SELL, 5.0, 1200.0, 16, 1150.0, 1280.0),
            (Side::SELL, 5.0, 1200.0, 16, 1150.0, 1300.0),
        ];

        for test_values in tests {
            let side = test_values.0;
            let points_per_tick = test_values.1;
            let current_price = test_values.2;
            let ticks_stop_loss = test_values.3;
            let current_price_dont_trigger_trailing = test_values.4;
            let current_price_trigger_stop_loss = test_values.5;

            let mut trailing_stop = TrailingStop::new(
                current_price,
                side,
                points_per_tick,
                ticks_stop_loss,
                13,
                5,
                2,
                1,
            );

            assert_eq!(false, trailing_stop.should_close_position(current_price_dont_trigger_trailing));
            assert_eq!(true, trailing_stop.should_close_position(current_price_trigger_stop_loss),
                       "should trigger the {side} stop_loss with given a current price of \
                       {current_price_trigger_stop_loss} but the internal stoploss is {}", trailing_stop.stop_loss_price);

            assert_eq!(trailing_stop.is_trailing_active(), false);
            assert_eq!(trailing_stop.is_finished(), true);
        }
    }
}