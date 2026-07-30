# Artifact-only isolated-environment rehearsal checklist

- [ ] Fresh or ephemeral supported x86-64 Linux environment
- [ ] No Pokeviewer source checkout or prior installed binary available
- [ ] Network disabled during verification
- [ ] Retained candidate archive and adjacent checksum provided read-only
- [ ] Outer SHA-256 passed before extraction
- [ ] Archive paths were safe
- [ ] All internal SHA-256 values passed
- [ ] Bundled CLI reported `pokeviewerctl 1.1.0`
- [ ] Candidate metadata matched the v1.1.0 contract
- [ ] Required release documents were present and readable
- [ ] No undocumented repair, rename, or source-tree shortcut used
- [ ] No unresolved release blocker
