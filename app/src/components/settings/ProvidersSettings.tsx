import { Loader, Select, Slider, Text } from "@mantine/core";
import { useEffect, useState } from "react";
import {
	useAvailableProviders,
	useSettings,
	useUpdateLLMModel,
	useUpdateLLMProvider,
	useUpdateSTTModel,
	useUpdateSTTProvider,
	useUpdateSTTTimeout,
} from "../../lib/queries";
import { LLM_MODELS, STT_MODELS } from "../../lib/modelOptions";
import { tauriAPI } from "../../lib/tauri";

// NOTE: This timeout is used by the Rust pipeline as a transcription request timeout.
// Keep this default aligned with backend fallbacks so "unset" settings don't lie.
const DEFAULT_STT_TIMEOUT = 10;

export function ProvidersSettings() {
	const { data: settings, isLoading: isLoadingSettings } = useSettings();
	const { data: availableProviders, isLoading: isLoadingProviders } =
		useAvailableProviders();

	// Wait for settings (source of truth) and provider list (for options)
	const isLoadingProviderData = isLoadingSettings || isLoadingProviders;
	const updateSTTProvider = useUpdateSTTProvider();
	const updateSTTModel = useUpdateSTTModel();
	const updateLLMProvider = useUpdateLLMProvider();
	const updateLLMModel = useUpdateLLMModel();
	const updateSTTTimeout = useUpdateSTTTimeout();

	const handleSTTProviderChange = (value: string | null) => {
		if (!value) return;
		// Save to local settings (Tauri) then notify overlay window to sync to server
		updateSTTProvider.mutate(value, {
			onSuccess: () => {
				// Reset model to first available when provider changes
				const models = STT_MODELS[value];
				const firstModel = models?.[0];
				if (firstModel) {
					updateSTTModel.mutate(firstModel.value);
				}
				tauriAPI.emitSettingsChanged();
			},
		});
	};

	const handleSTTModelChange = (value: string | null) => {
		if (!value) return;
		updateSTTModel.mutate(value, {
			onSuccess: () => {
				tauriAPI.emitSettingsChanged();
			},
		});
	};

	const handleLLMProviderChange = (value: string | null) => {
		if (!value) return;
		// Save to local settings (Tauri) then notify overlay window to sync to server
		updateLLMProvider.mutate(value, {
			onSuccess: () => {
				// Reset model to first available when provider changes
				const models = LLM_MODELS[value];
				const firstModel = models?.[0];
				if (firstModel) {
					updateLLMModel.mutate(firstModel.value);
				}
				tauriAPI.emitSettingsChanged();
			},
		});
	};

	const handleLLMModelChange = (value: string | null) => {
		if (!value) return;
		updateLLMModel.mutate(value, {
			onSuccess: () => {
				tauriAPI.emitSettingsChanged();
			},
		});
	};

	const handleSTTTimeoutChange = (value: number) => {
		// Save to local settings (Tauri) then notify overlay window to sync to server
		updateSTTTimeout.mutate(value, {
			onSuccess: () => {
				tauriAPI.emitSettingsChanged();
			},
		});
	};

	// Get the current timeout value from settings, falling back to default
	const currentTimeout = settings?.stt_timeout_seconds ?? DEFAULT_STT_TIMEOUT;

	// Local state for smooth slider dragging
	const [sliderValue, setSliderValue] = useState(currentTimeout);

	// Sync local state when server value changes
	useEffect(() => {
		setSliderValue(currentTimeout);
	}, [currentTimeout]);

	// Group providers by cloud/local for dropdown display
	const sttCloudProviders =
		availableProviders?.stt
			.filter((p) => !p.is_local)
			.map((p) => ({ value: p.value, label: p.label })) ?? [];
	const sttLocalProviders =
		availableProviders?.stt
			.filter((p) => p.is_local)
			.map((p) => ({ value: p.value, label: p.label })) ?? [];
	const sttProviderOptions = [
		{ group: "Cloud", items: sttCloudProviders },
		{ group: "Local", items: sttLocalProviders },
	];

	const llmCloudProviders =
		availableProviders?.llm
			.filter((p) => !p.is_local)
			.map((p) => ({ value: p.value, label: p.label })) ?? [];
	const llmLocalProviders =
		availableProviders?.llm
			.filter((p) => p.is_local)
			.map((p) => ({ value: p.value, label: p.label })) ?? [];
	const llmProviderOptions = [
		{ group: "Cloud", items: llmCloudProviders },
		{ group: "Local", items: llmLocalProviders },
	];

	// Get available models for the selected providers
	const sttModelOptions = settings?.stt_provider
		? (STT_MODELS[settings.stt_provider] ?? [])
		: [];
	const llmModelOptions = settings?.llm_provider
		? (LLM_MODELS[settings.llm_provider] ?? [])
		: [];

	return (
    <>
      {/* STT Provider */}
      <div className="settings-row">
        <div>
          <p className="settings-label">Speech-to-Text Provider</p>
          <p className="settings-description">Service for transcribing audio</p>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {isLoadingProviderData ? (
            <Loader size="sm" color="orange" />
          ) : (
            <Select
              data={sttProviderOptions}
              value={settings?.stt_provider ?? null}
              onChange={handleSTTProviderChange}
              placeholder="Select provider"
              withCheckIcon={false}
              disabled={
                sttCloudProviders.length === 0 && sttLocalProviders.length === 0
              }
              styles={{
                input: {
                  backgroundColor: "var(--bg-elevated)",
                  borderColor: "var(--border-default)",
                  color: "var(--text-primary)",
                  minWidth: 200,
                },
              }}
            />
          )}
        </div>
      </div>

      {/* STT Model - only show if provider has models */}
      {sttModelOptions.length > 0 && (
        <div className="settings-row">
          <div>
            <p className="settings-label">STT Model</p>
            <p className="settings-description">
              Model to use for transcription
            </p>
          </div>
          <Select
            data={sttModelOptions}
            value={settings?.stt_model ?? sttModelOptions[0]?.value ?? null}
            onChange={handleSTTModelChange}
            placeholder="Select model"
            withCheckIcon={false}
            styles={{
              input: {
                backgroundColor: "var(--bg-elevated)",
                borderColor: "var(--border-default)",
                color: "var(--text-primary)",
                minWidth: 200,
              },
            }}
          />
        </div>
      )}

      {/* LLM Provider */}
      <div className="settings-row">
        <div>
          <p className="settings-label">Language Model Provider</p>
          <p className="settings-description">AI service for text formatting</p>
        </div>
        <div style={{ display: "flex", alignItems: "center", gap: 8 }}>
          {isLoadingProviderData ? (
            <Loader size="sm" color="orange" />
          ) : (
            <Select
              data={llmProviderOptions}
              value={settings?.llm_provider ?? null}
              onChange={handleLLMProviderChange}
              placeholder="Select provider"
              withCheckIcon={false}
              disabled={
                llmCloudProviders.length === 0 && llmLocalProviders.length === 0
              }
              styles={{
                input: {
                  backgroundColor: "var(--bg-elevated)",
                  borderColor: "var(--border-default)",
                  color: "var(--text-primary)",
                  minWidth: 200,
                },
              }}
            />
          )}
        </div>
      </div>

      {/* LLM Model - only show if provider has models */}
      {llmModelOptions.length > 0 && (
        <div className="settings-row">
          <div>
            <p className="settings-label">Rewrite LLM Model</p>
            <p className="settings-description">
              LLM Model used to rewrite the transcription.
            </p>
          </div>
          <Select
            data={llmModelOptions}
            value={settings?.llm_model ?? llmModelOptions[0]?.value ?? null}
            onChange={handleLLMModelChange}
            placeholder="Select model"
            withCheckIcon={false}
            styles={{
              input: {
                backgroundColor: "var(--bg-elevated)",
                borderColor: "var(--border-default)",
                color: "var(--text-primary)",
                minWidth: 200,
              },
            }}
          />
        </div>
      )}

      {/* STT Timeout */}
      <div className="settings-row">
        <div style={{ flex: 1 }}>
          <p className="settings-label">STT Timeout</p>
          <p className="settings-description">
            Increase if nothing is getting transcribed
          </p>
          <div
            style={{
              marginTop: 12,
              display: "flex",
              alignItems: "center",
              gap: 12,
            }}
          >
            <Slider
              value={sliderValue}
              onChange={setSliderValue}
              onChangeEnd={handleSTTTimeoutChange}
              min={5}
              max={120}
              step={5}
              marks={[
                { value: 5, label: "5s" },
                { value: 30, label: "30s" },
                { value: 60, label: "60s" },
                { value: 120, label: "120s" },
              ]}
              styles={{
                root: { flex: 1 },
                track: { backgroundColor: "var(--bg-elevated)" },
                bar: { backgroundColor: "var(--accent-primary)" },
                thumb: { borderColor: "var(--accent-primary)" },
                markLabel: { color: "var(--text-secondary)", fontSize: 10 },
              }}
            />
            <Text size="xs" c="dimmed" style={{ minWidth: 32 }}>
              {Math.round(sliderValue)}s
            </Text>
          </div>
        </div>
      </div>
    </>
  );
}
