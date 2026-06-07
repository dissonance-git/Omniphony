import { app } from '../state.js';
import {
  setTrailInfoModalOpen, setEffectiveRenderInfoModalOpen,
  setOscInfoModalOpen, setAboutModalOpen, setRoomGeometryInfoModalOpen,
  setAdaptiveResamplingInfoModalOpen, setTelemetryGaugesInfoModalOpen,
  setRampModeInfoModalOpen, setVbapPositionInterpolationInfoModalOpen,
  setSpreadFromDistanceInfoModalOpen, setDistanceDiffuseInfoModalOpen,
  setEvaluationInfoModalOpen, setBackendInfoModalOpen,
  setDistanceModelInfoModalOpen, setInputInfoModalOpen,
  setInputClockInfoModalOpen, setInputLfeInfoModalOpen,
  setDrcInfoModalOpen, setHeatmapInfoModalOpen,
  setTelemetryGaugesOpen,
  setDisplaySectionOpen, setDrcSectionOpen, setAudioOutputSectionOpen, setInputSectionOpen, setRendererSectionOpen
} from '../modals.js';
import { closeAutoTuneWizardOnEscape } from '../auto-tune/wizard-ui.js';

function bindModalOpenClose({ buttonId, closeButtonId, modalId, open, close }) {
  const buttonEl = document.getElementById(buttonId);
  const closeButtonEl = document.getElementById(closeButtonId);
  const modalEl = document.getElementById(modalId);

  if (buttonEl) {
    buttonEl.addEventListener('click', open);
  }

  if (closeButtonEl) {
    closeButtonEl.addEventListener('click', close);
  }

  if (modalEl) {
    modalEl.addEventListener('click', (event) => {
      if (event.target === modalEl) {
        close();
      }
    });
  }
}

export function setupModalAndToggleListeners() {
  bindModalOpenClose({
    buttonId: 'spreadFromDistanceInfoBtn',
    closeButtonId: 'spreadFromDistanceInfoCloseBtn',
    modalId: 'spreadFromDistanceInfoModal',
    open: () => setSpreadFromDistanceInfoModalOpen(true),
    close: () => setSpreadFromDistanceInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'distanceDiffuseInfoBtn',
    closeButtonId: 'distanceDiffuseInfoCloseBtn',
    modalId: 'distanceDiffuseInfoModal',
    open: () => setDistanceDiffuseInfoModalOpen(true),
    close: () => setDistanceDiffuseInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'trailInfoBtn',
    closeButtonId: 'trailInfoCloseBtn',
    modalId: 'trailInfoModal',
    open: () => setTrailInfoModalOpen(true),
    close: () => setTrailInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'effectiveRenderInfoBtn',
    closeButtonId: 'effectiveRenderInfoCloseBtn',
    modalId: 'effectiveRenderInfoModal',
    open: () => setEffectiveRenderInfoModalOpen(true),
    close: () => setEffectiveRenderInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'oscInfoBtn',
    closeButtonId: 'oscInfoCloseBtn',
    modalId: 'oscInfoModal',
    open: () => setOscInfoModalOpen(true),
    close: () => setOscInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'roomGeometryInfoBtn',
    closeButtonId: 'roomGeometryInfoCloseBtn',
    modalId: 'roomGeometryInfoModal',
    open: () => setRoomGeometryInfoModalOpen(true),
    close: () => setRoomGeometryInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'adaptiveResamplingInfoBtn',
    closeButtonId: 'adaptiveResamplingInfoCloseBtn',
    modalId: 'adaptiveResamplingInfoModal',
    open: () => setAdaptiveResamplingInfoModalOpen(true),
    close: () => setAdaptiveResamplingInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'telemetryGaugesInfoBtn',
    closeButtonId: 'telemetryGaugesInfoCloseBtn',
    modalId: 'telemetryGaugesInfoModal',
    open: () => setTelemetryGaugesInfoModalOpen(true),
    close: () => setTelemetryGaugesInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'rampModeInfoBtn',
    closeButtonId: 'rampModeInfoCloseBtn',
    modalId: 'rampModeInfoModal',
    open: () => setRampModeInfoModalOpen(true),
    close: () => setRampModeInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'vbapPositionInterpolationInfoBtn',
    closeButtonId: 'vbapPositionInterpolationInfoCloseBtn',
    modalId: 'vbapPositionInterpolationInfoModal',
    open: () => setVbapPositionInterpolationInfoModalOpen(true),
    close: () => setVbapPositionInterpolationInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'evaluationInfoBtn',
    closeButtonId: 'evaluationInfoCloseBtn',
    modalId: 'evaluationInfoModal',
    open: () => setEvaluationInfoModalOpen(true),
    close: () => setEvaluationInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'backendInfoBtn',
    closeButtonId: 'backendInfoCloseBtn',
    modalId: 'backendInfoModal',
    open: () => setBackendInfoModalOpen(true),
    close: () => setBackendInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'distanceModelInfoBtn',
    closeButtonId: 'distanceModelInfoCloseBtn',
    modalId: 'distanceModelInfoModal',
    open: () => setDistanceModelInfoModalOpen(true),
    close: () => setDistanceModelInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'inputInfoBtn',
    closeButtonId: 'inputInfoCloseBtn',
    modalId: 'inputInfoModal',
    open: () => setInputInfoModalOpen(true),
    close: () => setInputInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'inputClockInfoBtn',
    closeButtonId: 'inputClockInfoCloseBtn',
    modalId: 'inputClockInfoModal',
    open: () => setInputClockInfoModalOpen(true),
    close: () => setInputClockInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'inputLfeInfoBtn',
    closeButtonId: 'inputLfeInfoCloseBtn',
    modalId: 'inputLfeInfoModal',
    open: () => setInputLfeInfoModalOpen(true),
    close: () => setInputLfeInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'drcInfoBtn',
    closeButtonId: 'drcInfoCloseBtn',
    modalId: 'drcInfoModal',
    open: () => setDrcInfoModalOpen(true),
    close: () => setDrcInfoModalOpen(false)
  });

  bindModalOpenClose({
    buttonId: 'heatmapInfoBtn',
    closeButtonId: 'heatmapInfoCloseBtn',
    modalId: 'heatmapInfoModal',
    open: () => setHeatmapInfoModalOpen(true),
    close: () => setHeatmapInfoModalOpen(false)
  });

  const aboutBtnEl = document.getElementById('aboutBtn');
  const aboutOpenAreaEl = document.getElementById('aboutOpenArea');
  const aboutCloseBtnEl = document.getElementById('aboutCloseBtn');
  const aboutModalEl = document.getElementById('aboutModal');

  if (aboutBtnEl) {
    aboutBtnEl.addEventListener('click', () => {
      setAboutModalOpen(true);
    });
  }

  if (aboutOpenAreaEl) {
    aboutOpenAreaEl.addEventListener('click', () => {
      setAboutModalOpen(true);
    });
    aboutOpenAreaEl.addEventListener('keydown', (event) => {
      if (event.key === 'Enter' || event.key === ' ') {
        event.preventDefault();
        setAboutModalOpen(true);
      }
    });
  }

  if (aboutCloseBtnEl) {
    aboutCloseBtnEl.addEventListener('click', () => {
      setAboutModalOpen(false);
    });
  }

  if (aboutModalEl) {
    aboutModalEl.addEventListener('click', (event) => {
      if (event.target === aboutModalEl) {
        setAboutModalOpen(false);
      }
    });
  }

  const telemetryGaugesToggleBtnEl = document.getElementById('telemetryGaugesToggleBtn');
  const displaySectionToggleBtnEl = document.getElementById('displaySectionToggleBtn');
  const drcSectionToggleBtnEl = document.getElementById('drcSectionToggleBtn');
  const audioOutputSectionToggleBtnEl = document.getElementById('audioOutputSectionToggleBtn');
  const inputSectionToggleBtnEl = document.getElementById('inputSectionToggleBtn');
  const rendererSectionToggleBtnEl = document.getElementById('rendererSectionToggleBtn');

  if (telemetryGaugesToggleBtnEl) {
    telemetryGaugesToggleBtnEl.addEventListener('click', () => {
      setTelemetryGaugesOpen(!app.telemetryGaugesOpen);
    });
  }

  if (displaySectionToggleBtnEl) {
    displaySectionToggleBtnEl.addEventListener('click', () => {
      setDisplaySectionOpen(!app.displaySectionOpen);
    });
  }

  if (drcSectionToggleBtnEl) {
    drcSectionToggleBtnEl.addEventListener('click', () => {
      setDrcSectionOpen(!app.drcSectionOpen);
    });
  }

  if (audioOutputSectionToggleBtnEl) {
    audioOutputSectionToggleBtnEl.addEventListener('click', () => {
      setAudioOutputSectionOpen(!app.audioOutputSectionOpen);
    });
  }

  if (inputSectionToggleBtnEl) {
    inputSectionToggleBtnEl.addEventListener('click', () => {
      setInputSectionOpen(!app.inputSectionOpen);
    });
  }

  if (rendererSectionToggleBtnEl) {
    rendererSectionToggleBtnEl.addEventListener('click', () => {
      setRendererSectionOpen(!app.rendererSectionOpen);
    });
  }

  document.addEventListener('keydown', (event) => {
    if (event.key === 'Escape') {
      setSpreadFromDistanceInfoModalOpen(false);
      setDistanceDiffuseInfoModalOpen(false);
      setEffectiveRenderInfoModalOpen(false);
      setAdaptiveResamplingInfoModalOpen(false);
      setTrailInfoModalOpen(false);
      setOscInfoModalOpen(false);
      setAboutModalOpen(false);
      setRoomGeometryInfoModalOpen(false);
      setTelemetryGaugesInfoModalOpen(false);
      setRampModeInfoModalOpen(false);
      setVbapPositionInterpolationInfoModalOpen(false);
      setEvaluationInfoModalOpen(false);
      setBackendInfoModalOpen(false);
      setDistanceModelInfoModalOpen(false);
      setInputInfoModalOpen(false);
      setInputClockInfoModalOpen(false);
      setInputLfeInfoModalOpen(false);
      setDrcInfoModalOpen(false);
      setHeatmapInfoModalOpen(false);
      closeAutoTuneWizardOnEscape();
    }
  });
}
