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
- [x] Camera track largest (or single) mass ~~too disorientating~~. But is required now ...
- [x] Check for collisions and conserve momentum.
- [x] Have deleted trails fade out (move the trail to a component without a rigid body and fade after in N frames - do this as another struct that adds a fade out param).
- [ ] Lessen rotational inertial, so it spins when struct (might need to reduce restitution too) (or just conserve rotational inertia ... ). Then combine when they settle?
- [ ] Document the functions so we can learn from it.
- [x] Add mouse interaction to add new bodies.
    - [x] Add velocity based on mouse drag.
- [ ] Add predicted path (based most influential body (Force) stored in the apply gravity system). - Create a PredictionTrail component.
- [ ] Fix pixels per meter (pretty sure it is initialization issue)