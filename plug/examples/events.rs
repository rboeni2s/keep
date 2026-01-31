use plug::prelude::*;


#[derive(Debug, Clone, Copy)]
enum PlayerBtn
{
    Play,
    Next,
    Previous,
}


/// Models a simple Mp3-Player
#[service]
struct Mp3Player
{
    // Subscribe to all `PlayerBtn` events
    #[event(PlayerBtn)]
    player_key_events: EventSubscriber<PlayerBtn>,
}


impl Mp3Player
{
    /// Will dispatch all events received by the event subscriber
    fn dispatch_events(&self)
    {
        // Dispatch all of the queued events...
        while let Some(event) = self.player_key_events.pop()
        {
            println!("{event:?} was pressed!");
        }
    }
}


/// Models a virtual human using the Mp3-Player
#[service]
struct VirtualHuman
{
    // Get a handle to the event emitter for `PlayerBtn` events
    #[layer]
    mp3_keys: EventEmitter<PlayerBtn>,
}


impl VirtualHuman
{
    /// Sends a `PlayerBtn` event
    fn press_btn(&self, btn: PlayerBtn)
    {
        // Sends the event to all subscribers of `PlayerBtn` events.
        // All of the events will be queued by each of the subscribers until
        // they are dispatched.
        self.mp3_keys.emit(btn);
    }
}


fn main()
{
    // Build the service registry
    let reg = build_reg![Mp3Player, VirtualHuman];

    // Press a few buttons
    let human = reg.get_unchecked::<VirtualHuman>();
    human.press_btn(PlayerBtn::Play);
    human.press_btn(PlayerBtn::Next);
    human.press_btn(PlayerBtn::Previous);

    // Now dispatch the events on the mp3 player
    reg.get_unchecked::<Mp3Player>().dispatch_events();
}
