# bevy_solar_system

## Build
```bash
cargo run --release
```

## Documentation
```bash
cargo doc --no-deps --open
```

# TODO
- [x] Add sprites (When over certain mass, make them planet like. Add a sun sprite)
- [x] Change sprite type based on mass.
- [ ] Add realtime print out.
- [ ] Break up entitites into plugins for modularity.
- [ ] Camera track largest mass.
- [x] Check for collisions and conserve momentum.
- [ ] Add predicted path (based on 1 body).
- [ ] Have deleted trails fade out (move the trail to a component without a rigid body and fade after in N frames).
- [ ] Document the functions so we can learn from it.