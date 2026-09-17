//! Local integration fixture encoder. Never used by the operator runtime.
#[path = "../tests/support/venue_fixture.rs"]
mod fixture;
fn main() {
    let input: fixture::VenueFixture = serde_json::from_reader(std::io::stdin()).unwrap();
    println!("{}", fixture::receipt(&input));
}
