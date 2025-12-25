import {
  ActionIcon,
  Checkbox,
  Group,
  NumberInput,
  SegmentedControl,
  Tooltip,
} from "@mantine/core";
import { notifications } from "@mantine/notifications";
import { FolderOpen } from "lucide-react";
import { useEffect, useState } from "react";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import {
  useRecordingsStats,
  useSettings,
  useUpdateMaxSavedRecordings,
  useUpdateTranscriptionRetention,
  useUpdateTranscriptionRetentionDeleteRecordings,
} from "../../lib/queries";
import {
  recordingsAPI,
  tauriAPI,
  type RewriteProgramPromptProfile,
  type TranscriptionRetentionUnit,
} from "../../lib/tauri";

type RequestLogsRetentionMode = "amount" | "time";
type RetentionMode = "amount" | "time";
type RetentionUnit = "days" | "hours";

const GLOBAL_ONLY_TOOLTIP =
  "This setting can only be changed in the Default profile";

export function DataSettings({
  editingProfileId,
}: {
  editingProfileId?: string;
}) {
  const { data: settings } = useSettings();

  const queryClient = useQueryClient();

  const updateRequestLogsRetention = useMutation({
    mutationFn: (params: {
      mode: RequestLogsRetentionMode;
      amount: number;
      unit: RetentionUnit;
      value: number;
    }) => (tauriAPI as any).updateRequestLogsRetention(params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["settings"] });
      queryClient.invalidateQueries({ queryKey: ["requestLogs"] });
    },
  });

  const updateMaxSavedRecordings = useUpdateMaxSavedRecordings();
  const updateTranscriptionRetention = useUpdateTranscriptionRetention();
  const updateTranscriptionRetentionDeleteRecordings =
    useUpdateTranscriptionRetentionDeleteRecordings();

  const recordingsStats = useRecordingsStats();

  const profiles = settings?.rewrite_program_prompt_profiles ?? [];
  const profile: RewriteProgramPromptProfile | null =
    editingProfileId && editingProfileId !== "default"
      ? profiles.find((p) => p.id === editingProfileId) ?? null
      : null;

  const isProfileScope = profile !== null;

  // ---------------------------------------------------------------------------
  // Logs retention
  // ---------------------------------------------------------------------------

  const settingsAny = settings as any;

  const logsRetentionModeFromSettings: RequestLogsRetentionMode =
    settingsAny?.request_logs_retention_mode ?? "amount";
  const logsRetentionAmountFromSettings =
    settingsAny?.request_logs_retention_amount ?? 10;
  const logsRetentionUnitFromSettings: RetentionUnit =
    settingsAny?.request_logs_retention_unit ?? "days";
  const logsRetentionValueFromSettings =
    settingsAny?.request_logs_retention_value ?? 7;

  const [logsRetentionDraft, setLogsRetentionDraft] = useState<{
    mode: RequestLogsRetentionMode;
    amount: number;
    unit: RetentionUnit;
    value: number;
  } | null>(null);

  useEffect(() => {
    // Drop draft once settings refresh from disk so we stay source-of-truth.
    setLogsRetentionDraft(null);
  }, [
    logsRetentionModeFromSettings,
    logsRetentionAmountFromSettings,
    logsRetentionUnitFromSettings,
    logsRetentionValueFromSettings,
  ]);

  const logsRetentionMode =
    logsRetentionDraft?.mode ?? logsRetentionModeFromSettings;
  const logsRetentionAmount =
    logsRetentionDraft?.amount ?? logsRetentionAmountFromSettings;
  const logsRetentionUnit =
    logsRetentionDraft?.unit ?? logsRetentionUnitFromSettings;
  const logsRetentionValue =
    logsRetentionDraft?.value ?? logsRetentionValueFromSettings;

  const commitLogsRetention = (next: {
    mode: RequestLogsRetentionMode;
    amount: number;
    unit: RetentionUnit;
    value: number;
  }) => {
    setLogsRetentionDraft(next);
    updateRequestLogsRetention.mutate(next);
  };

  // ---------------------------------------------------------------------------
  // Recordings retention (amount | time)
  // ---------------------------------------------------------------------------

  const recordingsRetentionModeFromSettings: RetentionMode =
    settingsAny?.recordings_retention_mode ?? "amount";
  const recordingsRetentionAmountFromSettings =
    settingsAny?.recordings_retention_amount ??
    settings?.max_saved_recordings ??
    50;
  const recordingsRetentionUnitFromSettings: RetentionUnit =
    settingsAny?.recordings_retention_unit ?? "days";
  const recordingsRetentionValueFromSettings =
    settingsAny?.recordings_retention_value ?? 0;

  const [recordingsRetentionDraft, setRecordingsRetentionDraft] = useState<{
    mode: RetentionMode;
    amount: number;
    unit: RetentionUnit;
    value: number;
  } | null>(null);

  useEffect(() => {
    setRecordingsRetentionDraft(null);
  }, [
    recordingsRetentionModeFromSettings,
    recordingsRetentionAmountFromSettings,
    recordingsRetentionUnitFromSettings,
    recordingsRetentionValueFromSettings,
  ]);

  const recordingsRetentionMode =
    recordingsRetentionDraft?.mode ?? recordingsRetentionModeFromSettings;
  const recordingsRetentionAmount =
    recordingsRetentionDraft?.amount ?? recordingsRetentionAmountFromSettings;
  const recordingsRetentionUnit =
    recordingsRetentionDraft?.unit ?? recordingsRetentionUnitFromSettings;
  const recordingsRetentionValue =
    recordingsRetentionDraft?.value ?? recordingsRetentionValueFromSettings;

  const updateRecordingsRetention = useMutation({
    mutationFn: (params: {
      mode: RetentionMode;
      amount: number;
      unit: RetentionUnit;
      value: number;
    }) => (tauriAPI as any).updateRecordingsRetention(params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["settings"] });
      queryClient.invalidateQueries({ queryKey: ["recordingsStats"] });
    },
  });

  const commitRecordingsRetention = (next: {
    mode: RetentionMode;
    amount: number;
    unit: RetentionUnit;
    value: number;
  }) => {
    setRecordingsRetentionDraft(next);
    updateRecordingsRetention.mutate(next);

    // Keep legacy key in sync for older builds / other call sites.
    if (next.mode === "amount") {
      updateMaxSavedRecordings.mutate(next.amount);
    }
  };

  const handleOpenRecordingsFolder = async () => {
    try {
      await recordingsAPI.openRecordingsFolder();
    } catch (e) {
      notifications.show({
        title: "Recordings",
        message: String(e),
        color: "red",
      });
    }
  };

  const recordingsSummary = (() => {
    const stats = recordingsStats.data;
    if (!stats) return null;
    if (typeof stats.count !== "number" || !Number.isFinite(stats.count))
      return null;
    if (typeof stats.bytes !== "number" || !Number.isFinite(stats.bytes))
      return null;

    const gb = stats.bytes / 1024 ** 3;
    return {
      count: Math.max(0, Math.round(stats.count)),
      gb,
    };
  })();

  // ---------------------------------------------------------------------------
  // Transcription retention (amount | time)
  // ---------------------------------------------------------------------------

  const transcriptionRetentionModeFromSettings: RetentionMode =
    settingsAny?.transcription_retention_mode ?? "time";
  const transcriptionRetentionAmountFromSettings =
    settingsAny?.transcription_retention_amount ?? 1000;

  const transcriptionRetentionUnitFromSettings: TranscriptionRetentionUnit =
    settings?.transcription_retention_unit ?? "days";
  const transcriptionRetentionValueFromSettings =
    settings?.transcription_retention_value ?? 0;
  const transcriptionRetentionDeleteRecordings =
    settings?.transcription_retention_delete_recordings ?? false;

  const [transcriptionRetentionDraft, setTranscriptionRetentionDraft] =
    useState<{
      mode: RetentionMode;
      amount: number;
      unit: TranscriptionRetentionUnit;
      value: number;
    } | null>(null);

  useEffect(() => {
    // Drop any draft once settings refresh from disk so we stay source-of-truth.
    setTranscriptionRetentionDraft(null);
  }, [
    transcriptionRetentionModeFromSettings,
    transcriptionRetentionAmountFromSettings,
    transcriptionRetentionUnitFromSettings,
    transcriptionRetentionValueFromSettings,
  ]);

  const transcriptionRetentionMode =
    transcriptionRetentionDraft?.mode ?? transcriptionRetentionModeFromSettings;
  const transcriptionRetentionAmount =
    transcriptionRetentionDraft?.amount ??
    transcriptionRetentionAmountFromSettings;
  const transcriptionRetentionUnit =
    transcriptionRetentionDraft?.unit ?? transcriptionRetentionUnitFromSettings;
  const transcriptionRetentionValue =
    transcriptionRetentionDraft?.value ??
    transcriptionRetentionValueFromSettings;

  const updateTranscriptionRetentionPolicy = useMutation({
    mutationFn: (params: {
      mode: RetentionMode;
      amount: number;
      unit: TranscriptionRetentionUnit;
      value: number;
    }) => (tauriAPI as any).updateTranscriptionRetentionPolicy(params),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["settings"] });
    },
  });

  const commitTranscriptionRetentionPolicy = (next: {
    mode: RetentionMode;
    amount: number;
    unit: TranscriptionRetentionUnit;
    value: number;
  }) => {
    setTranscriptionRetentionDraft(next);
    updateTranscriptionRetentionPolicy.mutate(next);

    // Keep the legacy/new time keys in sync when mode is time.
    if (next.mode === "time") {
      updateTranscriptionRetention.mutate({
        unit: next.unit,
        value: next.value,
      });
    }
  };

  const content = (
    <>
      <div className="settings-row">
        <div>
          <p className="settings-label">Logs retention</p>
          <p
            className="settings-description settings-description--single-line"
            title="Keep request logs for debugging. Default: store last 10."
          >
            Keep request logs for debugging.
          </p>
        </div>

        <Group gap={10} align="center" wrap="wrap">
          {logsRetentionMode === "amount" ? (
            <NumberInput
              value={logsRetentionAmount}
              onChange={(value) => {
                const nextAmount = typeof value === "number" ? value : 10;
                commitLogsRetention({
                  mode: "amount",
                  amount: nextAmount,
                  unit: logsRetentionUnit,
                  value: logsRetentionValue,
                });
              }}
              min={1}
              max={1000}
              step={1}
              clampBehavior="strict"
              disabled={isProfileScope}
              styles={{
                input: {
                  backgroundColor: "var(--bg-elevated)",
                  borderColor: "var(--border-default)",
                  color: "var(--text-primary)",
                  width: 140,
                },
              }}
            />
          ) : (
            <>
              <NumberInput
                value={logsRetentionValue}
                onChange={(value) => {
                  const nextValue = typeof value === "number" ? value : 7;
                  commitLogsRetention({
                    mode: "time",
                    amount: logsRetentionAmount,
                    unit: logsRetentionUnit,
                    value: nextValue,
                  });
                }}
                min={0}
                max={logsRetentionUnit === "hours" ? 36500 * 24 : 36500}
                step={logsRetentionUnit === "hours" ? 0.5 : 1}
                decimalScale={logsRetentionUnit === "hours" ? 2 : 0}
                clampBehavior="strict"
                disabled={isProfileScope}
                styles={{
                  input: {
                    backgroundColor: "var(--bg-elevated)",
                    borderColor: "var(--border-default)",
                    color: "var(--text-primary)",
                    width: 140,
                  },
                }}
              />

              <SegmentedControl
                value={logsRetentionUnit}
                onChange={(next) => {
                  const nextUnit = next === "hours" ? "hours" : "days";

                  const current =
                    typeof logsRetentionValue === "number"
                      ? logsRetentionValue
                      : 0;

                  // Preserve the underlying duration when switching units.
                  const nextValue =
                    current === 0
                      ? 0
                      : logsRetentionUnit === "days" && nextUnit === "hours"
                      ? current * 24
                      : logsRetentionUnit === "hours" && nextUnit === "days"
                      ? Math.round(current / 24)
                      : current;

                  commitLogsRetention({
                    mode: "time",
                    amount: logsRetentionAmount,
                    unit: nextUnit,
                    value: nextValue,
                  });
                }}
                data={[
                  { label: "Days", value: "days" },
                  { label: "Hours", value: "hours" },
                ]}
                disabled={isProfileScope}
                styles={{
                  root: {
                    backgroundColor: "var(--bg-elevated)",
                    border: "1px solid var(--border-default)",
                  },
                  label: {
                    color: "var(--text-primary)",
                  },
                }}
              />
            </>
          )}

          <SegmentedControl
            value={logsRetentionMode}
            onChange={(next) => {
              const mode =
                next === "time" ? ("time" as const) : ("amount" as const);
              commitLogsRetention({
                mode,
                amount: logsRetentionAmount,
                unit: logsRetentionUnit,
                value: logsRetentionValue,
              });
            }}
            data={[
              { label: "Amount", value: "amount" },
              { label: "Time", value: "time" },
            ]}
            disabled={isProfileScope}
            styles={{
              root: {
                backgroundColor: "var(--bg-elevated)",
                border: "1px solid var(--border-default)",
              },
              label: {
                color: "var(--text-primary)",
              },
            }}
          />
        </Group>
      </div>

      <div className="settings-row">
        <div>
          <p className="settings-label">Max recordings to save</p>
          <p
            className="settings-description settings-description--single-line"
            title={`Keep at most this many recordings on disk.${
              recordingsStats.isLoading
                ? " (Calculating storage…)"
                : recordingsSummary === null
                ? ""
                : ` (Currently saved ${
                    recordingsSummary.count
                  } recordings at ${recordingsSummary.gb.toFixed(2)} GB)`
            }`}
          >
            Keep at most this many recordings on disk.
            {recordingsStats.isLoading
              ? " (Calculating storage…)"
              : recordingsSummary === null
              ? ""
              : ` (Currently saved ${
                  recordingsSummary.count
                } recordings at ${recordingsSummary.gb.toFixed(2)} GB)`}
          </p>
        </div>
        <Group gap={8} align="center">
          <Tooltip label="Open recordings folder" withArrow position="top">
            <span>
              <ActionIcon
                variant="default"
                size={36}
                onClick={() => {
                  handleOpenRecordingsFolder().catch(console.error);
                }}
                aria-label="Open recordings folder"
                styles={{
                  root: {
                    backgroundColor: "var(--bg-elevated)",
                    borderColor: "var(--border-default)",
                    color: "var(--text-primary)",
                    height: 36,
                    width: 36,
                  },
                }}
              >
                <FolderOpen size={14} style={{ opacity: 0.75 }} />
              </ActionIcon>
            </span>
          </Tooltip>

          {recordingsRetentionMode === "amount" ? (
            <NumberInput
              value={recordingsRetentionAmount}
              onChange={(value) => {
                const nextAmount = typeof value === "number" ? value : 50;
                commitRecordingsRetention({
                  mode: "amount",
                  amount: nextAmount,
                  unit: recordingsRetentionUnit,
                  value: recordingsRetentionValue,
                });
              }}
              min={1}
              max={100000}
              step={10}
              clampBehavior="strict"
              disabled={isProfileScope}
              styles={{
                input: {
                  backgroundColor: "var(--bg-elevated)",
                  borderColor: "var(--border-default)",
                  color: "var(--text-primary)",
                  width: 140,
                },
              }}
            />
          ) : (
            <>
              <NumberInput
                value={recordingsRetentionValue}
                onChange={(value) => {
                  const nextValue = typeof value === "number" ? value : 0;
                  commitRecordingsRetention({
                    mode: "time",
                    amount: recordingsRetentionAmount,
                    unit: recordingsRetentionUnit,
                    value: nextValue,
                  });
                }}
                min={0}
                max={recordingsRetentionUnit === "hours" ? 36500 * 24 : 36500}
                step={recordingsRetentionUnit === "hours" ? 0.5 : 1}
                decimalScale={recordingsRetentionUnit === "hours" ? 2 : 0}
                clampBehavior="strict"
                disabled={isProfileScope}
                styles={{
                  input: {
                    backgroundColor: "var(--bg-elevated)",
                    borderColor: "var(--border-default)",
                    color: "var(--text-primary)",
                    width: 140,
                  },
                }}
              />

              <SegmentedControl
                value={recordingsRetentionUnit}
                onChange={(next) => {
                  const nextUnit = next === "hours" ? "hours" : "days";

                  const current =
                    typeof recordingsRetentionValue === "number"
                      ? recordingsRetentionValue
                      : 0;

                  // Preserve the underlying duration when switching units.
                  const nextValue =
                    current === 0
                      ? 0
                      : recordingsRetentionUnit === "days" &&
                        nextUnit === "hours"
                      ? current * 24
                      : recordingsRetentionUnit === "hours" &&
                        nextUnit === "days"
                      ? Math.round(current / 24)
                      : current;

                  commitRecordingsRetention({
                    mode: "time",
                    amount: recordingsRetentionAmount,
                    unit: nextUnit,
                    value: nextValue,
                  });
                }}
                data={[
                  { label: "Days", value: "days" },
                  { label: "Hours", value: "hours" },
                ]}
                disabled={isProfileScope}
                styles={{
                  root: {
                    backgroundColor: "var(--bg-elevated)",
                    border: "1px solid var(--border-default)",
                  },
                  label: {
                    color: "var(--text-primary)",
                  },
                }}
              />
            </>
          )}

          <SegmentedControl
            value={recordingsRetentionMode}
            onChange={(next) => {
              const mode = next === "time" ? "time" : "amount";
              commitRecordingsRetention({
                mode,
                amount: recordingsRetentionAmount,
                unit: recordingsRetentionUnit,
                value: recordingsRetentionValue,
              });
            }}
            data={[
              { label: "Amount", value: "amount" },
              { label: "Time", value: "time" },
            ]}
            disabled={isProfileScope}
            styles={{
              root: {
                backgroundColor: "var(--bg-elevated)",
                border: "1px solid var(--border-default)",
              },
              label: {
                color: "var(--text-primary)",
              },
            }}
          />
        </Group>
      </div>

      <div className="settings-row">
        <div>
          <p className="settings-label">Transcription retention</p>
          <p
            className="settings-description settings-description--single-line settings-description--tiny"
            title="Delete transcriptions older than this. Set to 0 to keep forever."
          >
            Delete transcriptions older than this (0 = forever).
          </p>
        </div>
        <Group gap={10} align="center" wrap="wrap">
          {transcriptionRetentionMode === "amount" ? (
            <NumberInput
              value={transcriptionRetentionAmount}
              onChange={(value) => {
                const nextAmount = typeof value === "number" ? value : 1000;
                commitTranscriptionRetentionPolicy({
                  mode: "amount",
                  amount: nextAmount,
                  unit: transcriptionRetentionUnit,
                  value: transcriptionRetentionValue,
                });
              }}
              min={1}
              max={100000}
              step={10}
              clampBehavior="strict"
              disabled={isProfileScope}
              styles={{
                input: {
                  backgroundColor: "var(--bg-elevated)",
                  borderColor: "var(--border-default)",
                  color: "var(--text-primary)",
                  width: 140,
                },
              }}
            />
          ) : (
            <>
              <NumberInput
                value={transcriptionRetentionValue}
                onChange={(value) => {
                  const next = typeof value === "number" ? value : 0;
                  commitTranscriptionRetentionPolicy({
                    mode: "time",
                    amount: transcriptionRetentionAmount,
                    unit: transcriptionRetentionUnit,
                    value: next,
                  });
                }}
                min={0}
                max={
                  transcriptionRetentionUnit === "hours" ? 36500 * 24 : 36500
                }
                step={transcriptionRetentionUnit === "hours" ? 0.5 : 1}
                decimalScale={transcriptionRetentionUnit === "hours" ? 2 : 0}
                clampBehavior="strict"
                disabled={isProfileScope}
                styles={{
                  input: {
                    backgroundColor: "var(--bg-elevated)",
                    borderColor: "var(--border-default)",
                    color: "var(--text-primary)",
                    width: 140,
                  },
                }}
              />

              <SegmentedControl
                value={transcriptionRetentionUnit}
                onChange={(next) => {
                  const nextUnit =
                    next === "hours" ? ("hours" as const) : ("days" as const);

                  const current =
                    typeof transcriptionRetentionValue === "number"
                      ? transcriptionRetentionValue
                      : 0;

                  // Preserve the underlying duration when switching units.
                  const nextValue =
                    current === 0
                      ? 0
                      : transcriptionRetentionUnit === "days" &&
                        nextUnit === "hours"
                      ? current * 24
                      : transcriptionRetentionUnit === "hours" &&
                        nextUnit === "days"
                      ? Math.round(current / 24)
                      : current;

                  commitTranscriptionRetentionPolicy({
                    mode: "time",
                    amount: transcriptionRetentionAmount,
                    unit: nextUnit,
                    value: nextValue,
                  });
                }}
                data={[
                  { label: "Days", value: "days" },
                  { label: "Hours", value: "hours" },
                ]}
                disabled={isProfileScope}
                styles={{
                  root: {
                    backgroundColor: "var(--bg-elevated)",
                    border: "1px solid var(--border-default)",
                  },
                  label: {
                    color: "var(--text-primary)",
                  },
                }}
              />
            </>
          )}

          <SegmentedControl
            value={transcriptionRetentionMode}
            onChange={(next) => {
              const mode = next === "time" ? "time" : "amount";
              commitTranscriptionRetentionPolicy({
                mode,
                amount: transcriptionRetentionAmount,
                unit: transcriptionRetentionUnit,
                value: transcriptionRetentionValue,
              });
            }}
            data={[
              { label: "Amount", value: "amount" },
              { label: "Time", value: "time" },
            ]}
            disabled={isProfileScope}
            styles={{
              root: {
                backgroundColor: "var(--bg-elevated)",
                border: "1px solid var(--border-default)",
              },
              label: {
                color: "var(--text-primary)",
              },
            }}
          />

          <Checkbox
            checked={transcriptionRetentionDeleteRecordings}
            onChange={(event) =>
              updateTranscriptionRetentionDeleteRecordings.mutate(
                event.currentTarget.checked
              )
            }
            disabled={
              isProfileScope ||
              (transcriptionRetentionMode === "time"
                ? transcriptionRetentionValue === 0
                : transcriptionRetentionAmount <= 0)
            }
            label="Also delete recordings"
            color="gray"
          />
        </Group>
      </div>
    </>
  );

  if (isProfileScope) {
    return (
      <Tooltip label={GLOBAL_ONLY_TOOLTIP} withArrow position="top-start">
        <div style={{ opacity: 0.5, cursor: "not-allowed" }}>
          <div style={{ pointerEvents: "none" }}>{content}</div>
        </div>
      </Tooltip>
    );
  }

  return content;
}
