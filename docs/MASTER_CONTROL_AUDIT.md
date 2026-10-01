# Master control message validation audit

Static source review on 1 October 2026, based on upstream `master` at
`ff7369ffa64eaa2ad41be0fe9f0879424a98afec`. The selected repair rejects invalid
room creation and option requests before they can change master state. The
wire version, field layout, and handling of requests accepted by the existing
encoder remain unchanged.

No game, application code, build, tests, probes, network experiments, or
dependency installation were run. Git and GitHub operations were used for
source retrieval and publication only. This report does not establish runtime
or network compatibility.

## Handler map before the repair

The shared codec is
[master_protocol](https://github.com/vladtrc/iw4L/blob/ff7369ffa64eaa2ad41be0fe9f0879424a98afec/crates/master_protocol/src/lib.rs).
The receiving paths are
[master service](https://github.com/vladtrc/iw4L/blob/ff7369ffa64eaa2ad41be0fe9f0879424a98afec/crates/iw4l-master/src/main.rs)
and [client bridge](https://github.com/vladtrc/iw4L/blob/ff7369ffa64eaa2ad41be0fe9f0879424a98afec/crates/net/src/transport/master.rs).
In the tables, integers are unsigned and big endian; IDs are fixed 16-byte
arrays; `s48`, for example, means a one-byte byte length followed by at most 48
UTF-8 bytes. `reason` is one byte, with two additional u64 values for content,
weapon, or class mismatch reasons.

| Source and entry point | Format and checks | Conversion and use |
| --- | --- | --- |
| Client or CLI to master `run_connection` | First bidirectional control frame must be `Hello`; repeat Hello and other non-request/non-relay frame kinds close the connection. Hello has u16 protocol, u32 game protocol, role 0..2, s64 build, s64 player name. | Name normalization trims, removes control characters, takes at most 16 Unicode characters, and defaults to Player. These are at most 64 UTF-8 bytes. Inner Hello protocol/game values are read but not separately gated here; the outer codec checks wire version and QUIC configuration uses the versioned ALPN. |
| Both control receivers `read_frame` | u32 stream length, 1..16384; exact read of that many bytes. Inner header is magic IW4M, u16 version 9, u8 kind, u64 request ID, u16 body length. Header requires at least 17 bytes and exact declared body length. | Allocate at most 16 KiB per frame before decoding. Invalid framing/codec errors do not reach a state handler. A complete frame is required; a peer leaving a stream open can keep an incomplete read pending. |
| Master `handle_request` | Request kind 1; body tag and fields listed below; `Reader::finish` rejects trailing bytes. | Fully decoded request enters the locked service state. Encoding checks for room options were missing from incoming request decoding. |
| Service to browser worker and CLI `rpc` | Response kind 2; browser expects RoomList for request ID 1; CLI RPC matches its request ID. | Browser converts adverts to UI records only after full decode. CLI prints list/status. |
| Service to session `handle_frame` | Response kind 2, RoomView kind 3, Closed kind 4, PeerEvent kind 5. Client rejects incoming Request; ignores Hello. Unknown codec kinds/tags fail. | RoomCreated/Joined/Updated and RoomView use `apply_room_view`; `SessionCore` accepts identity and newer revision/epoch before bridge mutation. Closed requires matching room and epoch. Other response bodies are ignored or, for Error, terminate the session. Session responses carrying views are not correlated with a pending request table. |
| Client or host to master datagram / uni stream / control Relay kind 6 | Relay envelope: u8 version 9, direction 1..4, 16-byte member for directions 2/3, opaque remaining payload. Minimum 2 or 18 bytes checked before slicing; payload capped at 1100 for datagrams, 256 KiB for streams. | `route_target` and `route_target_stream` establish room membership, host permission and target membership before forwarding; client-origin service directions are refused. The master does not parse game payloads. |
| Service to client `datagram_ingress` | Complete datagram envelope decoded before fragment reassembly. Host accepts ServiceToHost; joiner accepts ServiceToMember. Unknown, truncated, oversize or wrong-direction envelopes are ignored. | Per-member reassembler and bounded relay mailbox. Game fragment/packet internals are outside this audit. |
| Service to client `uni_ingress` | New receive stream, read to completion with a 262162-byte limit and deadline, then `decode_relay_stream`. Accepts ServiceToMember or HostToMember; other directions ignored. | Opaque payload queued to bootstrap lane. Bootstrap/game payload parsing is outside the selected service format. |
| Service to client control Relay | Outer frame bound also limits envelope to 16367 bytes. Host/joiner direction guard precedes bounded control mailbox. | Invalid envelope or wrong direction terminates this session. |

### Request fields and state handlers

Every field in this table is read through `Reader::take` or its numeric,
array, and string wrappers. Role and membership checks belong to the service
handlers rather than to the transport.

| Tag | Fields after tag | Handler and validation before state use |
| --- | --- | --- |
| 1 Status | none | Returns current protocol and limits; no room mutation. |
| 2 CreateRoom | s48 name, s64 map, s24 mode, u8 max_players, u8 content flags | `create_room` checks membership and room cap, then inserts room and membership and increments generation. Decoder allowed empty strings and all u8 capacities; encoder requires nonempty strings and 2..18 capacity. **Selected defect.** |
| 3 JoinRoom | room ID, u8 content flags | `join_room` checks room, lock, required flags, capacity. Repeat join to same room returns existing membership. Joining another room is refused. |
| 4 LeaveRoom | none | `leave_room` removes member or closes host room; repeated leave is harmless. |
| 5 SetOptions | s64 map, s24 mode, boolean joinable | `set_options` requires host; refuses changed map/mode during a match. In Gathering it writes options and bumps revision/generation before outgoing encoding. Empty map/mode accepted by decoder but refused by encoder. **Selected defect.** |
| 6 StartMatch | s64 map, s24 mode | `start_match` requires host; same running match returns existing view; changed map/mode during a match refused. In Gathering it writes options, increments nonzero epoch, sets Loading, and bumps revision/generation. Empty map/mode accepted by decoder but refused by encoder. **Selected defect.** |
| 7 EndMatch | room ID, u32 epoch | `end_match` checks room, nonzero matching epoch and match phase before returning to Gathering; stale or duplicate requests are acknowledged without mutation. |
| 8 CloseRoom | none | Host permission before closing room. |
| 9 ListRooms | none | Builds sorted adverts and truncates count to 128. |
| 10 HostWorldReady | room ID, u32 epoch, u64 map/weapons/classes hashes | Matching host, room, nonzero current epoch and in-match phase before Running and event publication. |
| 11 MapLoaded | u32 epoch, u64 map/weapons/classes hashes | `forward_to_host` derives sender's member ID from membership; host match state machine consumes event. |
| 12 BootstrapApplied | u32 epoch/bootstrap ID/snapshot sequence, u64 connection | Same member derivation; host queues typed bootstrap acknowledgement. Subsequent admission parsing is outside this repair. |
| 13 EnterMatch | member ID, u32 epoch/bootstrap ID, u64 connection ID, u32 client ID | `enter_match` requires sending host and target membership; forwards fields without checking current epoch. Receiver path described below. |
| 14 VoteToSkip | none | Membership-derived ID forwarded to host; receiver inserts into a HashSet, so duplicate votes do not multiply a member's vote. No wire epoch exists for this tag. |
| 15 AdmissionFailed | member ID, u32 epoch, reason | Host and target membership required; joiner receiver requires local member and nonzero matching epoch before failing admission. |

### Response and event fields

| Body | Format and decoder checks | Subsequent use |
| --- | --- | --- |
| Status | tag 1, u16 protocol/payload limit, u8 member limit | Typed status/CLI output; no arithmetic allocation from these fields. |
| RoomCreated / RoomJoined | tag 2/3, member ID, RoomView | Session identity and room view application. |
| RoomUpdated | tag 5, RoomView | Same view application. |
| RoomLeft / Ack | tag 4/10, no fields | No room payload; response ID can confirm closing. |
| RoomList | tag 9, u64 generation, u16 count <=128, adverts | Each advert: ID, s48 name, s64 map, s24 mode, u8 players/capacity, two booleans, u8 flags, u64 generation. Full decode precedes browser update. Semantic capacity/nonempty checks are absent here. |
| Error | tag 127 and known service error enum; missing-content error adds two flag bytes | Error returned; unknown enum rejected. |
| RoomView | room/host IDs, u64 revision, u32 epoch, phase 0..2, boolean joinable, u8 capacity/flags, s48 name, s64 map, s24 mode, u8 member count <=18, repeated ID+s64 name | Bounded local vector/map built before `finish`. Decoder does not enforce nonempty options, capacity range, unique members, or host presence. Session identity/revision gate precedes projection; it is not a full semantic validator for this payload. |
| Closed | room ID, u32 epoch, known close reason 0..2 | Matching identity checked before closure. |
| Peer events | event tags 1..6: HostWorldReady without room ID; MapLoaded and BootstrapApplied with member ID; EnterMatch with the request's fields; VoteToSkip with member ID; AdmissionFailed with member ID/epoch/reason | Numeric lengths and enum tags checked by codec. HostWorldReady/MapLoaded go to host state machine; BootstrapApplied queues an ack; AdmissionFailed checks local member and epoch. EnterMatch mutates bootstrap lane before the session's epoch guard. |

### Lengths, numbers, strings and transport guarantees

`Reader::take` uses `checked_add` and compares the end offset with the body
length before slicing or moving its cursor. Every u8/u16/u32/u64 and ID
therefore requires all its bytes. Count conversions u8/u16 to usize are
lossless on the supported desktop platforms; counts are bounded before vector
allocation. The u32 stream length is bounded before allocation. Relay offset
18 is used only after checking minimum length. The codec constructs local
values; incomplete strings, fields and trailing bytes do not partially update
service state.

Strings are length-prefixed UTF-8, not C strings. Their byte limits are checked
before `from_utf8`; a terminator is not required or appended. Embedded NUL is
valid UTF-8 and accepted by this codec; Hello player-name normalization removes
control characters. No new restriction on NUL, whitespace or character sets
is introduced by this repair. Binary IDs do not pass through the text ID
parser. Text ID parsing is a separate configuration/CLI path.

Transport evidence was read from the cached sources matching `Cargo.lock`:

- [Quinn 0.11.11 RecvStream source](https://docs.rs/quinn/0.11.11/src/quinn/recv_stream.rs.html):
  `read_exact` reads contiguous bytes and returns `FinishedEarly` when the
  stream ends too soon. It is not cancel-safe. Both control loops keep
  `read_owned_frame` in a pinned future between select iterations, so losing
  another select branch does not restart a partially consumed frame.
- [Quinn 0.11.11 read_to_end](https://docs.rs/quinn/0.11.11/quinn/struct.RecvStream.html#method.read_to_end):
  errors on exceeding its byte limit; can contain gaps if unordered reads were
  already made on that stream. These receive paths start with a fresh stream
  and do not perform earlier unordered reads. A deadline ends that stream's
  processing rather than continuing at a changed offset.
- [Quinn 0.11.11 datagram source](https://docs.rs/quinn/0.11.11/src/quinn/connection.rs.html):
  application datagrams are unreliable and unordered; each receive yields one
  datagram's Bytes. The pinned
  [quinn-proto receive queue](https://github.com/quinn-rs/quinn/blob/f650e0f213b01ea7bebd60023e5df7df65e5c2bd/quinn-proto/src/connection/datagrams.rs)
  queues complete datagrams and may evict old ones under buffer pressure.
  This does not guarantee valid application tags, lengths, strings or ranges.
  Stream reads also do not guarantee valid application contents. A hostile
  authenticated client can transmit bytes its normal application encoder
  would refuse.

## Concrete inputs and manual traces

These examples are descriptions only; none were executed or sent. The
notation makes every byte of a complete input explicit:

`H(N) = 49 57 34 4d 00 09 01 00 00 00 00 00 00 00 01 || BE16(N)`.
This is a request header with request ID 1. Its stream representation is
`BE32(17+N) || H(N) || body`. There is no trailing data.
Strings `R`, `a`, and `b` denote ordinary one-byte nonempty options accepted by
the existing encoder; the protocol validators do not require a catalog entry.

| Defect and exact body | N | Required starting state | Old path to wrong state |
| --- | --- | --- | --- |
| Empty CreateRoom name: `02 00 01 61 01 62 02 00` | 8 | Connected peer, no membership, room cap not reached | `decode_request_body` reads empty name, map a, mode b, capacity 2, flags 0; `finish` succeeds. `create_room` inserts room and membership and increments generation. `encode_room_view` later rejects empty name via `validate_advert`; control writer returns an error after mutation. Disconnect may clean up the room but does not make the earlier mutation or generation change disappear. |
| Invalid CreateRoom capacity: `02 01 52 01 61 01 62 ff 00` | 9 | Same | Decoder accepts capacity 255 as u8. `create_room` inserts that capacity and changes membership/generation. Outgoing view encoding rejects capacity outside 2..18. Capacities 0, 1 and 19 are other rejected-range cases with the same path. |
| Empty SetOptions map: `05 00 01 62 01` | 5 | Sending peer owns a Gathering room | Decoder accepts empty map and joinable true. `set_options` stores empty map and mode b, then `bump_room` changes revision/generation. View publication/RoomUpdated encoding fails because empty map is invalid. Existing members can lose control delivery through writer failure. |
| Empty StartMatch mode: `06 01 61 00` | 4 | Sending peer owns a Gathering room | Decoder accepts map a and empty mode. `start_match` stores them, increments epoch, sets Loading, and bumps revision/generation. Encoding the resulting view fails. The invalid request has already changed room phase and match identity. |

Empty map/mode in CreateRoom, empty mode in SetOptions, and empty map in
StartMatch follow the same respective traces. Oversize strings, invalid UTF-8,
invalid booleans, unknown tags, partial fields and trailing bytes already fail
in the codec before these service handlers.

### Separate confirmed event ordering issue

This finding is recorded separately and is not part of the selected room-option
repair. Let the local member be sixteen `11` bytes, live match epoch be 2,
and the bootstrap lane's entered fields be zero. An event body
`04 || [11 repeated 16 times] || 00 00 00 01 || 00 00 00 07 ||
00 00 00 00 00 00 00 09 || 00 00 00 03` has 37 bytes and means
EnterMatch for epoch 1, bootstrap 7, connection 9, client 3. Its control header
is `49 57 34 4d 00 09 05 || [00 repeated 8 times] || 00 25`, and
its stream prefix is `00 00 00 36` (54 bytes).

`decode_peer_event` accepts this well-formed event. In `apply_peer_event`, the
matching member passes the only bridge guard; `bootstrap.note_entered(7, 3)`
stores client 3 and bootstrap 7. Only afterwards does `SessionCore::apply`
reject epoch 1 because `epoch_applies(1, 2)` is false. The bootstrap lane remains
changed despite the session rejecting the event. The service's `enter_match`
also permits a room host to forward these stale fields to an existing member.
This is a confirmed premature state mutation, not a claim that a whole match
can be entered: downstream relay admission has additional bootstrap checks.
See [bridge event handling](https://github.com/vladtrc/iw4L/blob/ff7369ffa64eaa2ad41be0fe9f0879424a98afec/crates/net/src/transport/master.rs#L2670),
[bootstrap stores](https://github.com/vladtrc/iw4L/blob/ff7369ffa64eaa2ad41be0fe9f0879424a98afec/crates/net/src/transport/bootstrap.rs#L247),
and [session epoch guard](https://github.com/vladtrc/iw4L/blob/ff7369ffa64eaa2ad41be0fe9f0879424a98afec/crates/net/src/session_core.rs#L1157).

## Selected repair and static checks

The selected group is incoming room-option validation in request tags 2, 5,
and 6. Decode all fields into local variables, then reuse the exact
`validate_advert` / `validate_map_mode` predicates already used when encoding
these requests. Only after those checks succeed construct the RequestBody.
The enclosing `finish` check still runs before returning the ControlRequest.
Invalid option values cannot reach the service handlers or cause their room,
membership, phase, revision or generation mutations. The existing read-error
path closes the offending connection; its normal disconnect cleanup can
remove existing membership or close a room owned by that peer. Valid requests
retain their existing handlers.

Static acceptance cases: CreateRoom body
`02 01 52 01 61 01 62 02 00`; SetOptions body `05 01 61 01 62 01`;
StartMatch body `06 01 61 01 62`. Each supplies all fields, uses nonempty
strings within existing byte limits, and retains its original field values.
Capacity 18 and exactly 48/64/24 UTF-8 bytes remain valid. Capacity 19, a
49/65/25-byte string, malformed UTF-8, a missing final field, or extra trailing
bytes are refused. No new catalog, flag, string-terminator or protocol rules
are added.

All producers and receivers of these requests were inspected: host startup
CreateRoom in `session_main`, UpdateLobby/StartMatch in `handle_command`,
shared `encode_request`/`encode_stream_frame`, `decode_request` and
`decode_stream_payload`, master `read_frame`/`handle_request`, and
`create_room`/`set_options`/`start_match`. Outgoing RoomCreated, RoomUpdated and
RoomView paths already use `encode_room_view` with the same predicates. No
other network receiver directly constructs these decoded requests.

The repository has no service protocol unit tests or fixtures. Its
[approved test policy](https://github.com/vladtrc/iw4L/blob/ff7369ffa64eaa2ad41be0fe9f0879424a98afec/crates/approved_tests/README.md)
permits only named owner-approved gameplay scenarios and forbids permanent
unit tests elsewhere. The examples above preserve minimal regression inputs
without adding or running a new scenario. Validation is limited to source
inspection and diff checks; compilation, tests and network compatibility
remain unverified.

## Branch and open pull request context

The worktree was created from the freshly fetched upstream default branch,
`master`, at the baseline above, on `fix/master-control-validation-20261001`.
Open PRs at the initial review were 29, 33, 35, 38, 39, 40, 41, 42, 43, 44 and
45. PR 33 repairs game projectile snapshot decoding; PR 35 changes glass delta
lookup. Their formats and files differ from the selected master request codec.
No changes from those branches were included. No merge is requested or performed.
