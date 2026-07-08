# Goal

The goal for this application is to create a Rubik's solver with Bevy Engine.

# Functional Requirement

1. Given a cube configuration ..... return a step by step animation (that user can control to go to then next step, or go back) 
2. Support 3x3 cubes
3. User should only give minimal inputs (not all 54 colors) .... once it's enough .... we should let user know (so we need to allow user to input the initial cube configuration, reset the cube configuration, detect impossible configuration)
4. The UI should allow user to move, magnify/shrink, rotate, realign the whole cube for display purpose
5. User should be able to control the cube, like playing a real rubik cube

# Non-functional Requirement
1. a printable Cheat Sheet to guide user to finish a 3x3 cube systematically with illustration
2. the theory about each operation used in cheat sheet
3. support a local app and as a web app
4. a rust CLI command to start the app locally
5. the whole project should be able to package as a standard Rust package, and be able to publish at crates.io 
6. follow the best Rust coding practice

# Development Process
* Initialize the folder as a github repo 
* Choose Simplicity Solution over over engineering
* Make sure you have concrete plan for each step (without any ambuiguity before implementation)
* Always update the documentation for each commit.
* Follow TDD


# Clarification
1. "Minimal inputs … once it's enough, let the user know." This is the hardest requirement and the vaguest. What counts as "minimal"? What defines "enough"? A cube is only solvable-checkable once its state is fully determined — you generally can't infer unknown stickers. This needs a precise rule (e.g. "enter stickers until the state is fully constrained; validate parity/orientation").

=> When user input few sides of cube configuration, mathematically, if we can determine all 54 place's configuration, it means "enough" input, likewise, if it is impossible to have that configuration, we should let user know

2. Solver strategy is unspecified — and it silently couples to the cheat sheet. If the solver is optimal (Kociemba), its moves won't match a human cheat-sheet method. If it's layer-by-layer, the animation is the cheat sheet. The spec never says the animated solution and the cheat sheet must use the same method — but they almost certainly should. This one decision shapes half the codebase.

Nice catch, we should provide two solution method for user to decide
* optimal solver
* human cheat-sheet method

3. 2×2/3×3/4×4 are treated as equal effort — they aren't. 4×4 needs reduction + parity handling (materially harder). And the cheat sheet requirement is 3×3-only. Is that intentional?

=> Support 3x3 only for now for the entire app

4. "Detect impossible configuration" is listed but not scoped — it's real math (permutation parity + orientation sums). Fine, but it's a feature, not a checkbox.

=> see clarification 1, (and do the research to developp an algorithm is part of the step before implementation)

5. No success criteria, non-goals, or performance targets (solve time, FPS, mobile/touch input on web).

=> solve time, for compute, i think should be less than 1 second,  the app should be able to support 60 FPS, if work as app, we should support mobile/touch input as well ()

6. State-model gap: if the user manually moves the cube mid-solution, what happens to the animation/solver state? Undefined.

=> this is why we need the control to move back a frame in FR1, we cannot determine user error, once the initial cube configuration is set, user can always reset the configuration if the mess up
