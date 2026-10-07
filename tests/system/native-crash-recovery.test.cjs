const test = require("node:test");
const assert = require("node:assert/strict");
const fixture = require("../fixtures/native-production-crash-observations.json");
const { validateRecovery } = require("./native-crash-recovery.cjs");
const sample = () => structuredClone(fixture);

test("oracle accepts actual production fault, fresh instance and downstream client success", () => {
  const { events, fault } = sample();
  assert.equal(validateRecovery(events, fault).service_crash_restarted, true);
});
test("ordinary successful runtime cannot witness service crash recovery", () => {
  const { events, fault } = sample();
  const ordinary = events
    .filter((e) => e.event !== "native-completion" || e.selector !== 0)
    .filter((e) => e.event !== "native-process-terminal" || e.owner !== 1)
    .filter(
      (e) =>
        e.event !== "native-image" || e.selector !== 0 || e.generation !== 2,
    );
  assert.throws(() => validateRecovery(ordinary, fault));
});
test("oracle rejects wrong CPU, kernel-mode faults and the wrong operation boundary", () => {
  for (const change of [
    { cpu: 0 },
    { exceptionLevel: 1 },
    { operation: 2 },
    { repliesBeforeFault: 0 },
    { injectedPc: 4 },
  ]) {
    const { events, fault } = sample();
    assert.throws(() => validateRecovery(events, { ...fault, ...change }));
  }
});
test("oracle rejects stopped service, missing fresh identity and unrelated panic", () => {
  for (const mutate of [
    (events) => {
      events.find(
        (e) => e.event === "native-completion" && e.selector === 0,
      ).kind = "terminated";
    },
    (events) => {
      events.find(
        (e) => e.event === "native-image" && e.generation === 2,
      ).generation = 1;
    },
    (events) => {
      events.push({ event: "panic", status: "fail" });
    },
    (events) => {
      events.find(
        (e) => e.event === "native-completion" && e.selector === 1,
      ).code = 47;
    },
    (events) => {
      events.find((e) => e.words?.length === 6).words[3] = 1;
    },
    (events) => {
      events.find((e) => e.words?.length === 6).words[0] =
        "5426640009636886272";
    },
    (events) => {
      events.push(structuredClone(events.find((e) => e.words?.length === 6)));
    },
  ]) {
    const { events, fault } = sample();
    mutate(events);
    assert.throws(() => validateRecovery(events, fault));
  }
});
test("replacement preceding actual fault completion cannot pass the oracle", () => {
  const { events, fault } = sample();
  const completion = events.findIndex(
    (e) => e.event === "native-completion" && e.selector === 0,
  );
  const replacement = events.findIndex(
    (e) => e.event === "native-image" && e.generation === 2,
  );
  [events[completion], events[replacement]] = [
    events[replacement],
    events[completion],
  ];
  assert.throws(() => validateRecovery(events, fault));
});

test("completed recovery requires actual shutdown and zero retained resources", () => {
  const complete = require("../fixtures/native-production-crash-shutdown-observations.json");
  assert.equal(
    validateRecovery(complete.events, complete.fault, true).resource_leaks,
    0,
  );
  for (const mutate of [
    (events) =>
      events.splice(
        events.findIndex((e) => e.event === "native-boot"),
        1,
      ),
    (events) => {
      events.find((e) => e.event === "native-boot").live_domains = 1;
    },
    (events) => {
      events.find((e) => e.event === "native-boot").live_processes = 1;
    },
    (events) => {
      events.find((e) => e.event === "native-boot").frames_restored = false;
    },
    (events) => {
      events.find((e) => e.event === "native-boot").owners_released = false;
    },
    (events) => {
      events.find(
        (e) =>
          e.event === "native-completion" &&
          e.selector === 0 &&
          e.generation === 2,
      ).kind = "fault";
    },
    (events) => {
      events.find((e) => e.event === "native-boot").root_exit = 1;
    },
  ]) {
    const events = structuredClone(complete.events);
    mutate(events);
    assert.throws(() => validateRecovery(events, complete.fault, true));
  }
});
