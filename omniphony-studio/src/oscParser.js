function unwrapArg(arg) {
  return arg && typeof arg === 'object' && Object.prototype.hasOwnProperty.call(arg, 'value')
    ? arg.value
    : arg;
}

function toNumber(value) {
  const n = Number(value);
  return Number.isFinite(n) ? n : null;
}

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function sphericalToCartesian(azimuthDeg, elevationDeg, distance) {
  const az = (azimuthDeg * Math.PI) / 180;
  const el = (elevationDeg * Math.PI) / 180;
  const d = distance;

  return {
    x: d * Math.cos(el) * Math.cos(az),
    y: d * Math.sin(el),
    z: d * Math.cos(el) * Math.sin(az)
  };
}

function getOmniphonyCoordinateFormat(context) {
  return context && Number(context.omniphonyCoordinateFormat) === 1 ? 1 : 0;
}

function findIdInAddress(parts) {
  const anchors = ['source', 'sources', 'object', 'obj', 'track', 'channel'];
  const reserved = new Set(['position', 'pos', 'xyz', 'aed', 'spherical', 'polar', 'angles', 'remove', 'delete', 'off']);

  for (let i = 0; i < parts.length - 1; i += 1) {
    if (anchors.includes(parts[i])) {
      const candidate = parts[i + 1];
      if (!reserved.has(candidate)) {
        return candidate;
      }
    }
  }

  return null;
}

function mapCartesianByAddress(parts, position) {
  return position;
}

function parseJsonString(raw) {
  if (typeof raw !== 'string') {
    return null;
  }

  try {
    return JSON.parse(raw);
  } catch {
    return null;
  }
}

function parseOmniphonyConfigMessage(parts, args) {
  if (!parts.includes('omniphony') || !parts.includes('config')) {
    return null;
  }

  if (parts.length === 3 && parts[2] === 'speakers') {
    const count = toNumber(args[0]);
    if (count === null) {
      return null;
    }

    return {
      type: 'config:speakers:count',
      count: Math.max(0, Math.floor(count))
    };
  }

  if (parts.length === 4 && parts[2] === 'speaker') {
    const index = toNumber(parts[3]);
    if (index === null || index < 0) {
      return null;
    }

    const [nameRaw, azRaw, elRaw, distanceRaw, spatializeRaw] = args;
    const azimuth = toNumber(azRaw);
    const elevation = toNumber(elRaw);
    const distance = toNumber(distanceRaw);
    if (azimuth === null || elevation === null || distance === null) {
      return null;
    }

    const position = sphericalToCartesian(azimuth, elevation, distance);
    const spatialize = toNumber(spatializeRaw);

    return {
      type: 'config:speaker',
      index: Math.floor(index),
      name: String(nameRaw ?? `spk-${index}`),
      azimuthDeg: azimuth,
      elevationDeg: elevation,
      distanceM: distance,
      spatialize: spatialize === null ? 1 : spatialize !== 0 ? 1 : 0,
      position
    };
  }

  return null;
}

function parseOmniphonySpatialFrame(parts, args, context) {
  if (parts.length !== 3 || parts[0] !== 'omniphony' || parts[1] !== 'spatial' || parts[2] !== 'frame') {
    return null;
  }

  const samplePos = toNumber(args[0]);
  const hasGeneration = args.length >= 4;
  const objectCount = toNumber(args[hasGeneration ? 2 : 1]);
  if (samplePos === null || objectCount === null) {
    return null;
  }

  const coordinateFormat = toNumber(args[hasGeneration ? 3 : 2]) === 1 ? 1 : 0;
  if (context) {
    context.omniphonyCoordinateFormat = coordinateFormat;
  }

  return {
    type: 'spatial:frame',
    samplePos: Math.floor(samplePos),
    objectCount: Math.max(0, Math.floor(objectCount)),
    coordinateFormat
  };
}

function parseOmniphonyObjectPosition(parts, args, context) {
  if (!parts.includes('omniphony') || !parts.includes('object')) {
    return null;
  }

  const id = findIdInAddress(parts);
  if (!id) {
    return null;
  }

  const mode = parts.includes('aed') ? 'aed' : parts.includes('xyz') ? 'xyz' : null;
  if (!mode) {
    return null;
  }

  const a = toNumber(args[0]);
  const b = toNumber(args[1]);
  const c = toNumber(args[2]);
  if (a === null || b === null || c === null) {
    return null;
  }

  // /xyz | /aed payload after divergence removal:
  // [pos0, pos1, pos2, speaker_idx, gain, priority, ramp_duration, gen, name]
  // Fall back to the previous slot for backward compatibility.
  let nameRaw = args[8];
  if (typeof nameRaw !== 'string') {
    nameRaw = args[9];
  }
  const name = typeof nameRaw === 'string' && nameRaw.trim() ? nameRaw.trim() : null;

  let payload;
  if (mode === 'aed') {
    const coordinateFormat = getOmniphonyCoordinateFormat(context);
    if (coordinateFormat === 1) {
      payload = {
        x: 0,
        y: 0,
        z: 0,
        coordMode: 'polar',
        azimuthDeg: a,
        elevationDeg: b,
        distanceM: c
      };
    } else {
      const cart = sphericalToCartesian(a, b, c);
      payload = {
        x: clamp(cart.x, -1, 1),
        y: clamp(cart.y, -1, 1),
        z: clamp(cart.z, -1, 1)
      };
    }
  } else {
    const mapped = mapCartesianByAddress(parts, { x: a, y: b, z: c });
    payload = {
      x: clamp(mapped.x, -1, 1),
      y: clamp(mapped.y, -1, 1),
      z: clamp(mapped.z, -1, 1),
      coordMode: 'cartesian',
      azimuthDeg: undefined,
      elevationDeg: undefined,
      distanceM: undefined
    };
  }

  const result = { type: 'update', id: String(id), position: payload };
  if (name !== null) {
    result.name = name;
  }
  return result;
}

function parseOmniphonyObjectSize(parts, args) {
  if (!parts.includes('omniphony') || !parts.includes('object') || !parts.includes('size')) {
    return null;
  }
  const id = findIdInAddress(parts);
  if (!id) {
    return null;
  }
  const w = toNumber(args[0]);
  const d = toNumber(args[1]);
  const h = toNumber(args[2]);
  if (w === null || d === null || h === null) {
    return null;
  }
  return {
    type: 'update',
    id: String(id),
    size: {
      w: clamp(w, 0, 1),
      d: clamp(d, 0, 1),
      h: clamp(h, 0, 1)
    }
  };
}

function parseOmniphonyDomainState(parts, args) {
  if (parts.length < 3 || parts[0] !== 'omniphony' || parts[1] !== 'state') {
    return null;
  }

  const raw = args[0];
  if (parts.length === 3) {
    if (parts[2] === 'renderer') {
      const value = parseJsonString(raw);
      return value ? { type: 'state:renderer', value } : null;
    }
    if (parts[2] === 'layout') {
      const value = parseJsonString(raw);
      return value ? { type: 'state:layout', value } : null;
    }
    if (parts[2] === 'speakers') {
      const value = parseJsonString(raw);
      return value ? { type: 'state:speakers', value } : null;
    }
    if (parts[2] === 'audio') {
      const value = parseJsonString(raw);
      return value ? { type: 'state:audio', value } : null;
    }
    if (parts[2] === 'input') {
      const value = parseJsonString(raw);
      return value ? { type: 'state:input', value } : null;
    }
    if (parts[2] === 'session') {
      const value = parseJsonString(raw);
      return value ? { type: 'state:session', value } : null;
    }
    if (parts[2] === 'capabilities') {
      const value = parseJsonString(raw);
      return value ? { type: 'state:capabilities', value } : null;
    }
  }

  if (parts.length === 3 && parts[2] === 'loudness') {
    const value = parseJsonString(raw);
    return value ? { type: 'state:loudness', value } : null;
  }

  return null;
}

function parseOmniphonyRealtimeState(parts, args) {
  if (parts.length !== 4 || parts[0] !== 'omniphony' || parts[1] !== 'state' || parts[2] !== 'realtime') {
    return null;
  }

  if (parts[3] === 'master_gain') {
    const value = toNumber(args[0]);
    const seq = toNumber(args[1]);
    if (value === null || seq === null) {
      return null;
    }
    return { type: 'state:realtime:master_gain', value, seq: Math.floor(seq) };
  }

  if (parts[3] === 'speaker_gain') {
    const id = toNumber(args[0]);
    const value = toNumber(args[1]);
    const seq = toNumber(args[2]);
    if (id === null || value === null || seq === null) {
      return null;
    }
    return {
      type: 'state:realtime:speaker_gain',
      id: String(Math.floor(id)),
      value: clamp(value, 0, 2),
      seq: Math.floor(seq)
    };
  }

  if (parts[3] === 'object_gain') {
    const id = typeof args[0] === 'string' ? args[0] : null;
    const value = toNumber(args[1]);
    const seq = toNumber(args[2]);
    if (!id || value === null || seq === null) {
      return null;
    }
    return {
      type: 'state:realtime:object_gain',
      id,
      value: clamp(value, 0, 2),
      seq: Math.floor(seq)
    };
  }

  return null;
}

function parseOmniphonyStateMessage(parts, args) {
  const parsedDomain = parseOmniphonyDomainState(parts, args);
  if (parsedDomain) {
    return parsedDomain;
  }

  const parsedRealtime = parseOmniphonyRealtimeState(parts, args);
  if (parsedRealtime) {
    return parsedRealtime;
  }

  if (parts.length === 3 && parts[0] === 'omniphony' && parts[1] === 'state' && parts[2] === 'latency') {
    const value = toNumber(args[0]);
    return value === null ? null : { type: 'state:latency', value };
  }

  if (parts.length === 3 && parts[0] === 'omniphony' && parts[1] === 'state' && parts[2] === 'latency_instant') {
    const value = toNumber(args[0]);
    return value === null ? null : { type: 'state:latency:instant', value };
  }

  if (parts.length === 3 && parts[0] === 'omniphony' && parts[1] === 'state' && parts[2] === 'latency_control') {
    const value = toNumber(args[0]);
    return value === null ? null : { type: 'state:latency:control', value };
  }

  if (parts.length === 3 && parts[0] === 'omniphony' && parts[1] === 'state' && parts[2] === 'latency_target') {
    const value = toNumber(args[0]);
    return value === null ? null : { type: 'state:latency:target', value };
  }

  if (parts.length === 3 && parts[0] === 'omniphony' && parts[1] === 'state' && parts[2] === 'latency_downstream') {
    const value = toNumber(args[0]);
    return value === null ? null : { type: 'state:latency:downstream', value };
  }

  if (parts.length === 3 && parts[0] === 'omniphony' && parts[1] === 'state' && parts[2] === 'resample_ratio') {
    const value = toNumber(args[0]);
    return value === null ? null : { type: 'state:resample_ratio', value };
  }

  if (parts.length === 4 && parts[0] === 'omniphony' && parts[1] === 'state' && parts[2] === 'config' && parts[3] === 'saved') {
    const value = toNumber(args[0]);
    return value === null ? null : { type: 'state:config:saved', saved: value !== 0 };
  }

  if (parts.length === 5 && parts[0] === 'omniphony' && parts[1] === 'state' && parts[4] === 'gain') {
    const kind = parts[2];
    if (!['object', 'speaker'].includes(kind)) {
      return null;
    }

    const id = parts[3];
    const gain = toNumber(args[0]);
    if (!id || gain === null) {
      return null;
    }

    return {
      type: kind === 'speaker' ? 'state:speaker:gain' : 'state:object:gain',
      id: String(id),
      gain: clamp(gain, 0, 2)
    };
  }

  if (parts.length === 5 && parts[0] === 'omniphony' && parts[1] === 'state' && parts[4] === 'mute') {
    const kind = parts[2];
    if (!['object', 'speaker'].includes(kind)) {
      return null;
    }

    const id = parts[3];
    const muted = toNumber(args[0]);
    if (!id || muted === null) {
      return null;
    }

    return {
      type: kind === 'speaker' ? 'state:speaker:mute' : 'state:object:mute',
      id: String(id),
      muted: muted !== 0
    };
  }

  return null;
}

function parseObjectGainsMessage(parts, args) {
  const meterIndex = parts.indexOf('meter');
  if (meterIndex === -1 || parts.length <= meterIndex + 3) {
    return null;
  }

  const meterKind = parts[meterIndex + 1];
  const meterId = parts[meterIndex + 2];
  const suffix = parts[meterIndex + 3];

  if (meterKind !== 'object' || suffix !== 'gains' || !meterId) {
    return null;
  }

  return {
    type: 'meter:object:gains',
    id: String(meterId),
    gains: args.map((value) => clamp(toNumber(value) ?? 0, 0, 1))
  };
}

function parseMeterMessage(parts, args) {
  const meterIndex = parts.indexOf('meter');
  if (meterIndex === -1 || parts.length <= meterIndex + 2) {
    return null;
  }

  const meterKind = parts[meterIndex + 1];
  const meterId = parts[meterIndex + 2];
  if (!['object', 'speaker'].includes(meterKind) || !meterId) {
    return null;
  }

  const peakDbfs = clamp(toNumber(args[0]) ?? -100, -100, 0);
  const rmsDbfs = clamp(toNumber(args[1]) ?? -100, -100, 0);

  return {
    type: meterKind === 'object' ? 'meter:object' : 'meter:speaker',
    id: String(meterId),
    peakDbfs,
    rmsDbfs
  };
}

function parseOscMessage(oscMsg, context = null) {
  const address = String(oscMsg?.address || '');
  const parts = address.split('/').filter(Boolean).map((p) => p.toLowerCase());
  const args = (oscMsg?.args || []).map(unwrapArg);

  const parsedOmniphonyConfig = parseOmniphonyConfigMessage(parts, args);
  if (parsedOmniphonyConfig) {
    return parsedOmniphonyConfig;
  }

  const parsedOmniphonySpatialFrame = parseOmniphonySpatialFrame(parts, args, context);
  if (parsedOmniphonySpatialFrame) {
    return parsedOmniphonySpatialFrame;
  }

  const parsedOmniphonyObjectPosition = parseOmniphonyObjectPosition(parts, args, context);
  if (parsedOmniphonyObjectPosition) {
    return parsedOmniphonyObjectPosition;
  }

  const parsedOmniphonyObjectSize = parseOmniphonyObjectSize(parts, args);
  if (parsedOmniphonyObjectSize) {
    return parsedOmniphonyObjectSize;
  }

  const parsedOmniphonyState = parseOmniphonyStateMessage(parts, args);
  if (parsedOmniphonyState) {
    return parsedOmniphonyState;
  }

  if (parts.includes('meter')) {
    const parsedGains = parseObjectGainsMessage(parts, args);
    if (parsedGains) {
      return parsedGains;
    }
    return parseMeterMessage(parts, args);
  }

  const isRemove = parts.some((p) => ['remove', 'delete', 'off'].includes(p));
  if (isRemove) {
    const idFromAddress = findIdInAddress(parts);
    const idFromArg = args.length > 0 ? String(args[0]) : null;
    const id = idFromArg || idFromAddress;
    return id ? { type: 'remove', id } : null;
  }

  const idFromAddress = findIdInAddress(parts);
  const hasSphericalHint = parts.some((p) => ['aed', 'spherical', 'polar', 'angles'].includes(p));
  const hasPositionHint = parts.some((p) => ['position', 'xyz', 'pos'].includes(p));

  let id = idFromAddress;
  let numericArgs = args.map(toNumber).filter((n) => n !== null);

  if (!id && args.length >= 4) {
    id = String(args[0]);
    numericArgs = args.slice(1).map(toNumber).filter((n) => n !== null);
  }

  if (!id || numericArgs.length < 3) {
    return null;
  }

  let position;
  if (hasSphericalHint) {
    const [azimuth, elevation, distance] = numericArgs;
    position = sphericalToCartesian(azimuth, elevation, distance);
  } else if (hasPositionHint || numericArgs.length >= 3) {
    const [x, y, z] = numericArgs;
    position = { x, y, z };
  }

  if (!position) {
    return null;
  }

  const mappedPosition = mapCartesianByAddress(parts, position);

  return {
    type: 'update',
    id,
    position: {
      x: clamp(mappedPosition.x, -1, 1),
      y: clamp(mappedPosition.y, -1, 1),
      z: clamp(mappedPosition.z, -1, 1)
    }
  };
}

module.exports = {
  clamp,
  parseOscMessage,
  sphericalToCartesian
};
