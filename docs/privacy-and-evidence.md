# Privacy and public evidence

Pokeviewer is intended for a child, but the public repository must contain no
information about that child. Evidence should prove device behavior while
revealing as little personal or device-identifying information as possible.

## Never publish

- the child's name, face, voice, school, routine, or account information;
- home address, precise location, identifiable interiors, or location metadata;
- credentials, tokens, Wi-Fi details, private repository URLs, or private host
  paths;
- USB serial identifiers, device MAC addresses, or full unredacted device
  enumeration output;
- raw logs when a smaller sanitized excerpt proves the requirement; or
- photographs containing unrelated people, documents, screens, or reflections.

## Evidence procedure

1. Capture only the screen, board area, measurement, or terminal lines needed.
2. Disable location metadata when taking a photograph.
3. Redact identifiers at the source before saving or uploading the artifact.
4. Give each artifact the firmware commit, content-pack hash, hardware revision,
   test case, and local timestamp only when those fields are required.
5. Review the rendered artifact and its metadata before committing or uploading.
6. Prefer synthetic test inputs and generated screenshots over household
   photographs.

Redaction must be irreversible in the published artifact. Overlaying a box in
an editable document is not sufficient.

## Repository and release checks

Pull-request and release reviews must check:

- documentation examples use placeholders rather than real identifiers;
- CI logs and uploaded artifacts do not collect prohibited fields;
- screenshots and photographs have been visually reviewed;
- image metadata has been removed where applicable; and
- diagnostics default to the minimum data needed for recovery.

If prohibited information is published, remove the public artifact, rotate any
exposed credential, and replace the evidence with a sanitized version. Do not
preserve sensitive material merely to keep a stable link.
