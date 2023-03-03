# Location

The Location component is used instead of the Transform to represent your location in the universe. This location breaks your position into two parts: your sector coordinates and your local coordinates.

### Coordinates

Your sector coordinates are used to determine which sector you are in. Each one sector represents 10,000 blocks of space.  The local coordinates are used to determine where in this sector you are. These will be bounded between [-5000, 5000].  Any number outside of this range will cause a change in the sector coordinates.

### Transform/Location relationship

Due to the physics engine using Transforms natively but the rest of the game using Location, both must be kept in sync with each other. Because of this, both are safe to use for moving an object around, but for large space traversal the Location component should be preferred. In general, use the location component to control position, not the transform.