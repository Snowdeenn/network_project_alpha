use crate::simulation::components::*;
use legion::*;
use std::time::Duration;

#[system(for_each)]
#[filter(component::<Player>())]
pub fn dash(
    velo: &mut Velocity,
    dash: &mut Dash,
    state: &InputState,
    #[resource] delta_time: &Duration,
) {
    let new_state = match dash.0 {
        DashState::Idle => {
            if state.dash {
                velo.dx *= 5.0;
                velo.dy *= 5.0;
                DashState::Dashing(Duration::from_millis(20))
            } else {
                DashState::Idle
            }
        }

        DashState::Dashing(d) => {
            let remaining = d.saturating_sub(*delta_time);
            if remaining.is_zero() {
                DashState::Cooldown(Duration::from_secs(2))
            } else {
                DashState::Dashing(remaining)
            }
        }

        DashState::Cooldown(d) => {
            let remaining = d.saturating_sub(*delta_time);
            if remaining.is_zero() {
                DashState::Idle
            } else {
                DashState::Cooldown(remaining)
            }
        }
    };
    dash.0 = new_state;
}
