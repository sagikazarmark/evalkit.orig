> **📦 Archived on 2026-04-23** — superseded by [Evalkit vs. Verda: Final Evaluation Report](output.md). Kept for historical reference.

evalkit and verda are two, low-level, generic eval libraries.
Their main purpose is to provide the low-level constructs to run evals, most commonly used for testing and improving AI agents. BUT: from a design principle perspective, the library is independent of AI terminology and constraints. It should be able to serve any other use cases as well.

evalkit is a newer implementation based on a more detailed specification and the result of a better planning process (you will find both in the directory)

verda is slightly more mature, but the design process was more ad-hoc

Design principles:

- Stable abstraction that works the same for various use cases
- Simplicity: no overly complex APIs for minimal gain
- First class tracing support is vital

Tasks:
- Come up with a list of (test) use cases (at least 5-10) that you can validate the library API and implementation against
- Check out the prior research
- Do your own research about evals in general, but also look at the various available AI eval and test libraries (there is more and more by the day)
- create an ideal abstraction that works for all of defined use cases
- THEN: look at the code in the two projects
- Analyze the available abstractions, form an opinion based on your research and information
- Look at the implementations
- Test the libraries against your test cases (would it work?)
- Compare the ideal specification to what the libraries offers

SOme use cases of my own:
- Brain dump agent (segment and classsify thoughts): improve the agent so it's better at classification

Make a suggestion which library should be continued. What can one learn from the other. The "discarded" library: is there anything to preserve/migrate to the other one?
Any other improvements?
