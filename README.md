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
- [x] Add realtime print out.
- [x] Break up entitites into plugins for modularity.
- [ ] Camera track largest mass.
- [x] Check for collisions and conserve momentum.
- [ ] Add predicted path (based on 1 body).
- [x] Have deleted trails fade out (move the trail to a component without a rigid body and fade after in N frames - do this as another struct that adds a fade out param).
- [ ] Lessen rotational inertial, so it spins when struct (might need to reduce restitution too) (or just conserve rotational inertia ... ). Then combine when they settle?
- [ ] Document the functions so we can learn from it.
- [ ] Add mouse interaction to add new bodies.