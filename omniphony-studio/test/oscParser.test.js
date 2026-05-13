const test = require('node:test');
const assert = require('node:assert/strict');

const { parseOscMessage, sphericalToCartesian, clamp } = require('../src/oscParser');

function msg(address, args) {
  return { address, args };
}

test('clamp clamps values', () => {
  assert.equal(clamp(2, -1, 1), 1);
  assert.equal(clamp(-2, -1, 1), -1);
  assert.equal(clamp(0.2, -1, 1), 0.2);
});

test('sphericalToCartesian converts degrees to xyz', () => {
  const { x, y, z } = sphericalToCartesian(0, 0, 1);
  assert.ok(Math.abs(x - 1) < 1e-6);
  assert.ok(Math.abs(y - 0) < 1e-6);
  assert.ok(Math.abs(z - 0) < 1e-6);
});

test('parses cartesian with id in args', () => {
  const parsed = parseOscMessage(msg('/source/position', ['7', 0.2, -0.1, 0.4]));
  assert.deepEqual(parsed, {
    type: 'update',
    id: '7',
    position: { x: 0.2, y: -0.1, z: 0.4 }
  });
});

test('returns null for malformed or insufficient args', () => {
  assert.equal(parseOscMessage(msg('/source/position', ['x', 'y'])), null);
  assert.equal(parseOscMessage(msg('/source/position', [1, 2])), null);
  assert.equal(parseOscMessage(msg('/source/position', [])), null);
});

test('parses cartesian with id in address', () => {
  const parsed = parseOscMessage(msg('/source/5/position', [0.2, 0.1, -0.4]));
  assert.deepEqual(parsed, {
    type: 'update',
    id: '5',
    position: { x: 0.2, y: 0.1, z: -0.4 }
  });
});

test('clamps cartesian positions to [-1, 1]', () => {
  const parsed = parseOscMessage(msg('/source/1/position', [2, -3, 0.5]));
  assert.deepEqual(parsed, {
    type: 'update',
    id: '1',
    position: { x: 1, y: -1, z: 0.5 }
  });
});

test('parses spherical aed with id in address', () => {
  const parsed = parseOscMessage(msg('/source/9/aed', [90, 0, 1]));
  assert.equal(parsed.type, 'update');
  assert.equal(parsed.id, '9');
  assert.ok(Math.abs(parsed.position.x - 0) < 1e-6);
  assert.ok(Math.abs(parsed.position.y - 0) < 1e-6);
  assert.ok(Math.abs(parsed.position.z - 1) < 1e-6);
});

test('parses remove even with reserved keywords in address', () => {
  const parsed = parseOscMessage(msg('/object/remove', ['99']));
  assert.deepEqual(parsed, { type: 'remove', id: '99' });
});

test('parses remove with id in args', () => {
  const parsed = parseOscMessage(msg('/source/remove', [12]));
  assert.deepEqual(parsed, { type: 'remove', id: '12' });
});

test('parses remove with id in address', () => {
  const parsed = parseOscMessage(msg('/source/12/remove', []));
  assert.deepEqual(parsed, { type: 'remove', id: '12' });
});

test('parses meter and gains', () => {
  const meter = parseOscMessage(msg('/omniphony/meter/object/3', [-2, -10]));
  assert.deepEqual(meter, {
    type: 'meter:object',
    id: '3',
    peakDbfs: -2,
    rmsDbfs: -10
  });

  const gains = parseOscMessage(msg('/omniphony/meter/object/3/gains', [1.2, 0.5, -1]));
  assert.deepEqual(gains, {
    type: 'meter:object:gains',
    id: '3',
    gains: [1, 0.5, 0]
  });
});

test('clamps meter values to [-100, 0]', () => {
  const meter = parseOscMessage(msg('/omniphony/meter/speaker/2', [5, -200]));
  assert.deepEqual(meter, {
    type: 'meter:speaker',
    id: '2',
    peakDbfs: 0,
    rmsDbfs: -100
  });
});

test('parses omniphony object xyz mapping', () => {
  const parsed = parseOscMessage(msg('/omniphony/object/7/xyz', [0.2, 0.3, 0.4]));
  assert.deepEqual(parsed, {
    type: 'update',
    id: '7',
    position: { x: 0.2, y: 0.3, z: 0.4, coordMode: 'cartesian', azimuthDeg: undefined, elevationDeg: undefined, distanceM: undefined }
  });
});

test('parses omniphony spatial frame and preserves explicit xyz decoding', () => {
  const ctx = { omniphonyCoordinateFormat: 0 };
  const frame = parseOscMessage(msg('/omniphony/spatial/frame', [1024, 77, 3, 1]), ctx);
  assert.deepEqual(frame, {
    type: 'spatial:frame',
    samplePos: 1024,
    objectCount: 3,
    coordinateFormat: 1
  });
  assert.equal(ctx.omniphonyCoordinateFormat, 1);

  const parsed = parseOscMessage(msg('/omniphony/object/7/xyz', [0.2, 0.3, 0.4]), ctx);
  assert.equal(parsed.type, 'update');
  assert.equal(parsed.id, '7');
  assert.deepEqual(parsed.position, { x: 0.2, y: 0.3, z: 0.4, coordMode: 'cartesian', azimuthDeg: undefined, elevationDeg: undefined, distanceM: undefined });
});

test('parses omniphony object aed in polar mode', () => {
  const ctx = { omniphonyCoordinateFormat: 0 };
  const frame = parseOscMessage(msg('/omniphony/spatial/frame', [1024, 77, 3, 1]), ctx);
  assert.equal(frame.coordinateFormat, 1);

  const parsed = parseOscMessage(msg('/omniphony/object/7/aed', [90, 0, 1]), ctx);
  assert.deepEqual(parsed, {
    type: 'update',
    id: '7',
    position: { x: 0, y: 0, z: 0, coordMode: 'polar', azimuthDeg: 90, elevationDeg: 0, distanceM: 1 }
  });
});

test('parses legacy 3-argument omniphony spatial frame for backwards compatibility', () => {
  const ctx = { omniphonyCoordinateFormat: 0 };
  const frame = parseOscMessage(msg('/omniphony/spatial/frame', [1024, 3, 1]), ctx);
  assert.deepEqual(frame, {
    type: 'spatial:frame',
    samplePos: 1024,
    objectCount: 3,
    coordinateFormat: 1
  });
  assert.equal(ctx.omniphonyCoordinateFormat, 1);
});

test('parses serialized renderer and loudness domains', () => {
  assert.deepEqual(
    parseOscMessage(msg('/omniphony/state/renderer', [
      JSON.stringify({
        masterGain: 0.75,
        distanceModel: 'inverse-square',
        roomRatio: { width: 1, length: 2, height: 3 },
        spread: { min: 0.1, max: 0.9 },
        distanceDiffuse: { enabled: true, threshold: 0.5, curve: 1.2 }
      })
    ])),
    {
      type: 'state:renderer',
      value: {
        masterGain: 0.75,
        distanceModel: 'inverse-square',
        roomRatio: { width: 1, length: 2, height: 3 },
        spread: { min: 0.1, max: 0.9 },
        distanceDiffuse: { enabled: true, threshold: 0.5, curve: 1.2 }
      }
    }
  );

  assert.deepEqual(
    parseOscMessage(msg('/omniphony/state/loudness', [
      JSON.stringify({ enabled: true, source: -24, gain: 0.8 })
    ])),
    {
      type: 'state:loudness',
      value: { enabled: true, source: -24, gain: 0.8 }
    }
  );
});

test('parses serialized layout and speakers domains', () => {
  assert.deepEqual(
    parseOscMessage(msg('/omniphony/state/layout', [
      JSON.stringify({
        radius_m: 1.5,
        speakers: [{ id: 0, name: 'L', azimuth: 30, elevation: 0, distance: 1, spatialize: true }]
      })
    ])),
    {
      type: 'state:layout',
      value: {
        radius_m: 1.5,
        speakers: [{ id: 0, name: 'L', azimuth: 30, elevation: 0, distance: 1, spatialize: true }]
      }
    }
  );

  assert.deepEqual(
    parseOscMessage(msg('/omniphony/state/speakers', [
      JSON.stringify({ speakers: [{ id: 0, gain: 0.7, delayMs: 2.5, muted: true }] })
    ])),
    {
      type: 'state:speakers',
      value: { speakers: [{ id: 0, gain: 0.7, delayMs: 2.5, muted: true }] }
    }
  );
});

test('parses realtime gain acknowledgements', () => {
  assert.deepEqual(
    parseOscMessage(msg('/omniphony/state/realtime/master_gain', [0.9, 12])),
    { type: 'state:realtime:master_gain', value: 0.9, seq: 12 }
  );

  assert.deepEqual(
    parseOscMessage(msg('/omniphony/state/realtime/speaker_gain', [6, 0.7, 3])),
    { type: 'state:realtime:speaker_gain', id: '6', value: 0.7, seq: 3 }
  );

  assert.deepEqual(
    parseOscMessage(msg('/omniphony/state/realtime/object_gain', ['7', 1.2, 5])),
    { type: 'state:realtime:object_gain', id: '7', value: 1.2, seq: 5 }
  );
});

test('parses omniphony state messages', () => {
  assert.deepEqual(
    parseOscMessage(msg('/omniphony/state/latency', [12.5])),
    { type: 'state:latency', value: 12.5 }
  );

  assert.deepEqual(
    parseOscMessage(msg('/omniphony/state/resample_ratio', [0.999])),
    { type: 'state:resample_ratio', value: 0.999 }
  );

  assert.deepEqual(
    parseOscMessage(msg('/omniphony/state/object/5/gain', [1.5])),
    { type: 'state:object:gain', id: '5', gain: 1.5 }
  );

  assert.deepEqual(
    parseOscMessage(msg('/omniphony/state/object/5/gain', [3])),
    { type: 'state:object:gain', id: '5', gain: 2 }
  );

  assert.deepEqual(
    parseOscMessage(msg('/omniphony/state/speaker/6/gain', [0.7])),
    { type: 'state:speaker:gain', id: '6', gain: 0.7 }
  );

  assert.deepEqual(
    parseOscMessage(msg('/omniphony/state/object/5/mute', [1])),
    { type: 'state:object:mute', id: '5', muted: true }
  );

  assert.deepEqual(
    parseOscMessage(msg('/omniphony/state/speaker/6/mute', [0])),
    { type: 'state:speaker:mute', id: '6', muted: false }
  );

  assert.deepEqual(
    parseOscMessage(msg('/omniphony/state/config/saved', [1])),
    { type: 'state:config:saved', saved: true }
  );
});
