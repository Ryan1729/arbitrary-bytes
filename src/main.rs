use web;

use player;

fn main() {
    let params = web::get_state_params();
    let state = player::EntireState::new(params);
    web::run(state);
}
