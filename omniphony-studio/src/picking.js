import * as THREE from 'three';
import { app, speakerMeshes, sourceMeshes, sourceLabels, sourceOutlines, sourceNames, speakerLabels, speakerBandBars } from './state.js';
import { scene, camera, renderer, controls } from './scene/setup.js';
import { raycaster, pointer } from './scene/materials.js';
import { speakerGizmo, distanceGizmo, cartesianGizmo } from './scene/gizmos.js';
import { normalizeAngleDeg, snapAngleDeg, sphericalToCartesianDeg, normalizedOmniphonyToScenePosition, sceneToOmniphonyCartesian } from './coordinates.js';
import { applySpeakerPolarEdit, applySpeakerCartesianEdit, applySpeakerSceneCartesianEdit, setSelectedSpeaker, renderSpeakerEditor } from './speakers.js';
import { setSelectedSource } from './sources.js';
import { channelPlacement, applyChannelDirection } from './controls/virtual-bed.js';
import { projectRayOntoAxis } from './input.js';
import { getCanvasClientRect, pointerEventToNdc as projectPointerEventToNdc } from './core/render/projection-service.js';

let boundCanvas = null;
// Marker scene position when a virtual-bed drag began, to distinguish a real
// drag (commit + send) from a plain click-select (no-op).
const virtualBedDragStart = new THREE.Vector3();

function onPointerDown(event) {
  app.pointerDownPosition = { x: event.clientX, y: event.clientY };
  // A virtual-bed channel marker takes priority over speaker-gizmo drags.
  if (beginVirtualBedDrag(event) || beginSpeakerDrag(event)) {
    app.pointerDownPosition = null;
  }
}

function onPointerUp(event) {
  if (app.isDraggingVirtualBed && event.pointerId === app.draggingPointerId) {
    endVirtualBedDrag();
  }
  if (app.isDraggingSpeaker && event.pointerId === app.draggingPointerId) {
    endSpeakerDrag();
  }
  if (!app.pointerDownPosition) {
    return;
  }

  const dx = event.clientX - app.pointerDownPosition.x;
  const dy = event.clientY - app.pointerDownPosition.y;
  app.pointerDownPosition = null;

  if (Math.hypot(dx, dy) <= 6) {
    const hitSceneItem = selectSceneItemFromPointer(event);
    if (hitSceneItem) {
      return;
    }
    setSelectedSource(null);
    setSelectedSpeaker(null);
    updateControlsForEditMode();
  }
}

function onPointerMove(event) {
  if (app.isDraggingVirtualBed && event.pointerId === app.draggingPointerId) {
    updateVirtualBedDrag(event);
    return;
  }
  if (app.isDraggingSpeaker && event.pointerId === app.draggingPointerId) {
    updateSpeakerDrag(event);
  }
}

function onPointerCancel() {
  endVirtualBedDrag();
  endSpeakerDrag();
}

function onPointerLeave() {
  endVirtualBedDrag();
  endSpeakerDrag();
}

function onWheel(event) {
  if (app.activeEditMode !== 'polar' || app.selectedSpeakerIndex === null || !app.polarEditArmed) {
    return;
  }
  if (!event.ctrlKey && !event.shiftKey) {
    return;
  }
  event.preventDefault();
  event.stopPropagation();
  const prevZoom = controls.enableZoom;
  controls.enableZoom = false;

  const delta = -Math.sign(event.deltaY);
  const step = event.shiftKey ? 0.01 : 0.05;
  const next = Math.min(2.0, Math.max(0.2, app.dragDistance + delta * step));
  if (next === app.dragDistance) {
    return;
  }
  app.dragDistance = next;
  const pos = sphericalToCartesianDeg(app.dragAzimuthDeg, app.dragElevationDeg, app.dragDistance);
  const mesh = speakerMeshes[app.selectedSpeakerIndex];
  if (mesh) {
    mesh.position.set(pos.x, pos.y, pos.z);
  }
  const label = speakerLabels[app.selectedSpeakerIndex];
  if (label) {
    label.position.set(pos.x, pos.y + 0.12, pos.z);
  }
  const speaker = app.currentLayoutSpeakers[app.selectedSpeakerIndex];
  if (speaker) {
    applySpeakerSceneCartesianEdit(app.selectedSpeakerIndex, pos.x, pos.y, pos.z, false);
  }
  controls.enableZoom = prevZoom;
}

export function rebindPointerListeners() {
  const canvas = renderer.domElement;
  if (boundCanvas === canvas) {
    return;
  }
  if (boundCanvas) {
    boundCanvas.removeEventListener('pointerdown', onPointerDown);
    boundCanvas.removeEventListener('pointerup', onPointerUp);
    boundCanvas.removeEventListener('pointermove', onPointerMove);
    boundCanvas.removeEventListener('pointercancel', onPointerCancel);
    boundCanvas.removeEventListener('pointerleave', onPointerLeave);
    boundCanvas.removeEventListener('wheel', onWheel, true);
  }
  canvas.addEventListener('pointerdown', onPointerDown);
  canvas.addEventListener('pointerup', onPointerUp);
  canvas.addEventListener('pointermove', onPointerMove);
  canvas.addEventListener('pointercancel', onPointerCancel);
  canvas.addEventListener('pointerleave', onPointerLeave);
  canvas.addEventListener('wheel', onWheel, { passive: false, capture: true });
  boundCanvas = canvas;
}

function updateControlsForEditMode() {
  controls.enableZoom = true;
}

export function pointerEventToNdc(event) {
  projectPointerEventToNdc(event, getCanvasClientRect(renderer.domElement), pointer);
}

export function getPickableSceneTargets() {
  const sourceTargets = [
    ...sourceLabels.values(),
    ...sourceMeshes.values(),
    ...sourceOutlines.values()
  ].filter((object) => object && object.visible !== false);
  const speakerTargets = [
    ...speakerMeshes,
    ...speakerLabels,
    ...speakerBandBars
  ].filter((object) => object && object.visible !== false);
  return [...sourceTargets, ...speakerTargets];
}

export function pickSpeakerFromIntersects(intersects) {
  for (const hit of intersects) {
    const object = hit.object;
    const speakerIdx = speakerMeshes.indexOf(object);
    if (speakerIdx >= 0) {
      setSelectedSource(null);
      setSelectedSpeaker(speakerIdx);
      return true;
    }

    const labelIdx = speakerLabels.indexOf(object);
    if (labelIdx >= 0) {
      setSelectedSource(null);
      setSelectedSpeaker(labelIdx);
      return true;
    }

    const bandIdx = speakerBandBars.indexOf(object);
    if (bandIdx >= 0) {
      setSelectedSource(null);
      setSelectedSpeaker(bandIdx);
      return true;
    }
  }
  return false;
}

export function selectSceneItemFromPointer(event) {
  pointerEventToNdc(event);
  raycaster.setFromCamera(pointer, camera);
  const intersects = raycaster.intersectObjects(getPickableSceneTargets(), false);

  if (pickSpeakerFromIntersects(intersects)) {
    return true;
  }

  for (const hit of intersects) {
    const object = hit.object;
    const sourceId = object?.userData?.sourceId;
    if (sourceId !== undefined && sourceId !== null) {
      setSelectedSource(sourceId);
      setSelectedSpeaker(null);
      updateControlsForEditMode();
      return true;
    }

    const speakerIdx = speakerMeshes.indexOf(object);
    if (speakerIdx >= 0) {
      setSelectedSource(null);
      setSelectedSpeaker(speakerIdx);
      return true;
    }

    const labelIdx = speakerLabels.indexOf(object);
    if (labelIdx >= 0) {
      setSelectedSource(null);
      setSelectedSpeaker(labelIdx);
      return true;
    }
  }

  return false;
}

export function beginSpeakerDrag(event) {
  if (app.selectedSpeakerIndex === null) {
    return false;
  }
  pointerEventToNdc(event);
  raycaster.setFromCamera(pointer, camera);

  if (app.activeEditMode === 'polar' && app.polarEditArmed) {
    const gizmoHits = raycaster.intersectObjects([speakerGizmo.ring, speakerGizmo.arc], false);
    if (gizmoHits.length === 0) {
      return false;
    }
    const hit = gizmoHits[0].object;
    app.dragMode = hit === speakerGizmo.ring ? 'azimuth' : 'elevation';
    app.isDraggingSpeaker = true;
    app.draggingPointerId = event.pointerId;
    app.dragAzimuthDelta = 1;
    app.dragElevationDelta = 1;
    controls.enabled = false;
    return true;
  }

  if (app.activeEditMode === 'cartesian' && app.cartesianEditArmed) {
    const handleHits = raycaster.intersectObjects(
      [cartesianGizmo.xHandle, cartesianGizmo.yHandle, cartesianGizmo.zHandle],
      false
    );
    if (handleHits.length === 0) {
      return false;
    }
    const axis = handleHits[0].object?.userData?.axis;
    if (!axis) {
      return false;
    }
    app.dragMode = 'cartesian';
    app.dragAxis = axis;
    app.dragAxisDirection.set(
      axis === 'x' ? 1 : 0,
      axis === 'y' ? 1 : 0,
      axis === 'z' ? 1 : 0
    );
    const mesh = speakerMeshes[app.selectedSpeakerIndex];
    if (mesh) {
      app.dragAxisOrigin.copy(mesh.position);
      app.dragSpeakerStartPosition.copy(mesh.position);
      app.dragAxisStartT = projectRayOntoAxis(
        raycaster.ray.origin,
        raycaster.ray.direction,
        app.dragAxisOrigin,
        app.dragAxisDirection
      );
    }
    app.isDraggingSpeaker = true;
    app.draggingPointerId = event.pointerId;
    controls.enabled = false;
    return true;
  }

  return false;
}

export function updateSpeakerDrag(event) {
  if (!app.isDraggingSpeaker || app.selectedSpeakerIndex === null) {
    return;
  }
  pointerEventToNdc(event);
  raycaster.setFromCamera(pointer, camera);

  if (app.dragMode === 'azimuth') {
    const plane = new THREE.Plane(new THREE.Vector3(0, 1, 0), 0);
    const hitPoint = new THREE.Vector3();
    if (raycaster.ray.intersectPlane(plane, hitPoint)) {
      app.dragAzimuthDeg = (Math.atan2(hitPoint.z, hitPoint.x) * 180) / Math.PI;
      app.dragAzimuthDeg = normalizeAngleDeg(app.dragAzimuthDeg);
      const radial = Math.sqrt(hitPoint.x * hitPoint.x + hitPoint.z * hitPoint.z);
      const delta = (radial - app.dragDistance) / app.dragDistance;
      app.dragAzimuthDelta = delta;
      if (delta >= 0 && delta <= 0.1) {
        app.dragAzimuthDeg = snapAngleDeg(app.dragAzimuthDeg, 1, 0.5);
      } else if (delta > 0.1) {
        app.dragAzimuthDeg = snapAngleDeg(app.dragAzimuthDeg, 5, 2.5);
      }
    }
  } else if (app.dragMode === 'elevation') {
    const azRad = (app.dragAzimuthDeg * Math.PI) / 180;
    const dir = new THREE.Vector3(Math.cos(azRad), 0, Math.sin(azRad));
    const normal = new THREE.Vector3().crossVectors(dir, new THREE.Vector3(0, 1, 0)).normalize();
    const plane = new THREE.Plane().setFromNormalAndCoplanarPoint(normal, new THREE.Vector3(0, 0, 0));
    const hitPoint = new THREE.Vector3();
    if (raycaster.ray.intersectPlane(plane, hitPoint)) {
      const planar = Math.sqrt(hitPoint.x * hitPoint.x + hitPoint.z * hitPoint.z);
      app.dragElevationDeg = (Math.atan2(hitPoint.y, planar) * 180) / Math.PI;
      app.dragElevationDeg = Math.max(-90, Math.min(90, app.dragElevationDeg));
      const radius = Math.sqrt(hitPoint.x * hitPoint.x + hitPoint.y * hitPoint.y + hitPoint.z * hitPoint.z);
      const delta = (radius - app.dragDistance) / app.dragDistance;
      app.dragElevationDelta = delta;
      if (delta >= 0 && delta <= 0.1) {
        app.dragElevationDeg = snapAngleDeg(app.dragElevationDeg, 1, 0.5);
      } else if (delta > 0.1) {
        app.dragElevationDeg = snapAngleDeg(app.dragElevationDeg, 5, 2.5);
      }
    }
  } else if (app.dragMode === 'cartesian') {
    const tNow = projectRayOntoAxis(
      raycaster.ray.origin,
      raycaster.ray.direction,
      app.dragAxisOrigin,
      app.dragAxisDirection
    );
    const delta = tNow - app.dragAxisStartT;
    const pos = app.dragSpeakerStartPosition.clone().add(app.dragAxisDirection.clone().multiplyScalar(delta));
    applySpeakerSceneCartesianEdit(app.selectedSpeakerIndex, pos.x, pos.y, pos.z, false);
    return;
  }

  const pos = sphericalToCartesianDeg(app.dragAzimuthDeg, app.dragElevationDeg, app.dragDistance);
  const mesh = speakerMeshes[app.selectedSpeakerIndex];
  if (mesh) {
    mesh.position.set(pos.x, pos.y, pos.z);
  }
  const label = speakerLabels[app.selectedSpeakerIndex];
  if (label) {
    label.position.set(pos.x, pos.y + 0.12, pos.z);
  }
  const speaker = app.currentLayoutSpeakers[app.selectedSpeakerIndex];
  if (speaker) {
    applySpeakerSceneCartesianEdit(app.selectedSpeakerIndex, pos.x, pos.y, pos.z, false);
  }
}

export function endSpeakerDrag() {
  if (!app.isDraggingSpeaker || app.selectedSpeakerIndex === null) {
    return;
  }
  app.isDraggingSpeaker = false;
  app.dragMode = null;
  app.dragAxis = null;
  app.draggingPointerId = null;
  controls.enabled = true;

  if (app.selectedSpeakerIndex !== null) {
    const idx = app.selectedSpeakerIndex;
    const speaker = app.currentLayoutSpeakers[idx];
    if (speaker) {
      const scenePosition = normalizedOmniphonyToScenePosition(speaker);
      applySpeakerSceneCartesianEdit(idx, scenePosition.x, scenePosition.y, scenePosition.z, true);
    }
  }
}

// ---------------------------------------------------------------------------
// Virtual-bed channel drag
//
// 2D-source markers are shown at their pure angle (the renderer pre-warps the
// object so the room warp cancels on display), so we read azimuth/elevation
// straight from the dragged marker's scene direction — no room-warp inversion.
// Distance is preserved; only `spatialize:true` channels are draggable (direct
// channels are anchored to their speaker). The new direction is committed on
// release; `updateSource` ignores the live stream for the dragged marker.
// ---------------------------------------------------------------------------

function beginVirtualBedDrag(event) {
  if (app.channelRenderMode === 'host') {
    return false;
  }
  pointerEventToNdc(event);
  raycaster.setFromCamera(pointer, camera);
  const markers = [...sourceMeshes.values()].filter((m) => m && m.visible !== false);
  const hits = raycaster.intersectObjects(markers, false);
  for (const hit of hits) {
    const id = hit.object?.userData?.sourceId;
    if (id === undefined || id === null) {
      continue;
    }
    const name = sourceNames.get(String(id));
    if (channelPlacement(name) !== 'virtual') {
      // Only virtualized channels can be repositioned; direct ones are pinned
      // to their speaker, and non-bed objects fall through to normal handling.
      continue;
    }
    app.isDraggingVirtualBed = true;
    app.draggingVirtualBedSourceId = String(id);
    app.draggingVirtualBedChannel = name;
    app.draggingPointerId = event.pointerId;
    virtualBedDragStart.copy(sourceMeshes.get(String(id)).position);
    setSelectedSpeaker(null);
    setSelectedSource(id);
    controls.enabled = false;
    return true;
  }
  return false;
}

function updateVirtualBedDrag(event) {
  if (!app.isDraggingVirtualBed) {
    return;
  }
  const mesh = sourceMeshes.get(app.draggingVirtualBedSourceId);
  if (!mesh) {
    return;
  }
  pointerEventToNdc(event);
  raycaster.setFromCamera(pointer, camera);
  // Project the cursor onto the sphere of the marker's current radius (its
  // closest approach to the origin), keeping distance fixed and giving an
  // intuitive "aim the channel" gesture.
  const radius = mesh.position.length() || 1;
  const o = raycaster.ray.origin;
  const d = raycaster.ray.direction;
  const t = -o.dot(d);
  const dir = new THREE.Vector3().copy(d).multiplyScalar(t).add(o);
  if (dir.lengthSq() < 1e-9) {
    dir.copy(mesh.position);
  }
  dir.normalize().multiplyScalar(radius);
  mesh.position.copy(dir);
  const label = sourceLabels.get(app.draggingVirtualBedSourceId);
  if (label) {
    label.position.set(dir.x, dir.y + 0.12, dir.z);
  }
}

function endVirtualBedDrag() {
  if (!app.isDraggingVirtualBed) {
    return;
  }
  const mesh = sourceMeshes.get(app.draggingVirtualBedSourceId);
  const name = app.draggingVirtualBedChannel;
  app.isDraggingVirtualBed = false;
  app.draggingVirtualBedSourceId = null;
  app.draggingVirtualBedChannel = null;
  app.draggingPointerId = null;
  controls.enabled = true;
  if (!mesh || !name) {
    return;
  }
  // A plain click (no real movement) selects the marker without re-sending.
  if (mesh.position.distanceTo(virtualBedDragStart) < 1e-4) {
    return;
  }
  // Scene direction → Omniphony angle (x=right, y=front, z=up). Distance is kept
  // by applyChannelDirection (only the angle changes).
  const omni = sceneToOmniphonyCartesian(mesh.position);
  const horizontal = Math.hypot(omni.x, omni.y);
  const azimuth = (Math.atan2(omni.x, omni.y) * 180) / Math.PI;
  const elevation = horizontal > 1e-6 ? (Math.atan2(omni.z, horizontal) * 180) / Math.PI : 0;
  applyChannelDirection(name, azimuth, elevation);
}

export function setupPointerListeners() {
  rebindPointerListeners();
}
