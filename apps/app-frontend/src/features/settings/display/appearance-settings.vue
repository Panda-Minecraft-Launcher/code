<script setup lang="ts">
import {
	AppearanceSettingsLayout,
	defineMessages,
	injectAuth,
	injectUserPreferences,
	provideAppearanceSettings,
	SegmentedControl,
	useSavable,
	useVIntl,
} from '@modrinth/ui'
import { platform } from '@tauri-apps/plugin-os'
import { computed, onBeforeUnmount, onMounted, watch } from 'vue'

import { useAppSettings } from '@/composables/use-app-settings.ts'
import { type ColorTheme, isDarkTheme, useTheme } from '@/composables/use-theme.ts'
import { injectAppSettingsModal } from '@/features/settings/app-settings-modal/context'
import { type AppSettings, get, set } from '@/helpers/settings.ts'

const theme = useTheme()
const appSettings = useAppSettings()
const auth = injectAuth()
const { updatePreferences } = injectUserPreferences()
const settingsModal = injectAppSettingsModal()
const os = platform()
const { formatMessage } = useVIntl()

const messages = defineMessages({
	sidebarPositionTitle: {
		id: 'app.appearance-settings.sidebar-position.title',
		defaultMessage: 'Sidebar position',
	},
	sidebarPositionDescription: {
		id: 'app.appearance-settings.sidebar-position.description',
		defaultMessage: 'Show the sidebar on the right or left.',
	},
	sidebarPositionRight: {
		id: 'app.appearance-settings.sidebar-position.right',
		defaultMessage: 'Right',
	},
	sidebarPositionLeft: {
		id: 'app.appearance-settings.sidebar-position.left',
		defaultMessage: 'Left',
	},
})

type AppearanceSettingsState = {
	theme: ColorTheme
	syncAcrossDevices: boolean
	advancedRendering: boolean
	nativeDecorations: boolean
	sidebarOnRight: boolean
}

function getAppearanceSettingsState(): AppearanceSettingsState {
	return {
		theme: theme.preferred,
		syncAcrossDevices: theme.syncAcrossDevices,
		advancedRendering: theme.advancedRendering,
		nativeDecorations: appSettings.nativeDecorations,
		sidebarOnRight: appSettings.sidebarOnRight,
	}
}

const { saved, current, changes, saving, hasChanges, reset, save } = useSavable(
	getAppearanceSettingsState,
	async (appearanceChanges) => {
		const value = current.value
		if (
			value.syncAcrossDevices &&
			auth.user.value &&
			(appearanceChanges.theme !== undefined || appearanceChanges.syncAcrossDevices !== undefined)
		) {
			await updatePreferences({
				appearance: value.theme === 'system' ? { auto: true } : { auto: false, theme: value.theme },
			})
		}

		const nextSettings: AppSettings = {
			...(await get()),
			theme: value.theme,
			sync_theme_across_devices: value.syncAcrossDevices,
			advanced_rendering: value.advancedRendering,
			native_decorations: value.nativeDecorations,
			sidebar_on_right: value.sidebarOnRight,
		}

		await set(nextSettings)
		if (isDarkTheme(value.theme)) {
			theme.preferredDark = value.theme
		}
		theme.preferred = value.theme
		theme.syncAcrossDevices = value.syncAcrossDevices
		theme.advancedRendering = value.advancedRendering
		appSettings.nativeDecorations = value.nativeDecorations
		appSettings.sidebarOnRight = value.sidebarOnRight
	},
)

const themeOptions = computed(() =>
	theme.options.filter(
		(option) => option !== 'retro' || appSettings.devMode || current.value.theme === 'retro',
	),
)

const preferredDarkTheme = computed(() =>
	isDarkTheme(current.value.theme) ? current.value.theme : theme.preferredDark,
)

function setTheme(value: ColorTheme): void {
	current.value.theme = value
}

function setSyncAcrossDevices(enabled: boolean): void {
	current.value.syncAcrossDevices = enabled
}

function setAdvancedRendering(enabled: boolean): void {
	current.value.advancedRendering = enabled
}

function setNativeDecorations(enabled: boolean): void {
	current.value.nativeDecorations = enabled
}

const sidebarPositions: boolean[] = [true, false]

function formatSidebarPosition(onRight: boolean): string {
	return formatMessage(onRight ? messages.sidebarPositionRight : messages.sidebarPositionLeft)
}

watch(
	[() => current.value.theme, () => saved.value.theme],
	([selectedTheme, savedTheme]) => {
		theme.preview = selectedTheme === savedTheme ? null : selectedTheme
	},
	{ immediate: true },
)

async function saveAppearanceSettings(): Promise<void> {
	try {
		await save()
	} catch {
		return
	}
}

onMounted(() => {
	settingsModal.registerUnsavedChangesController({
		hasChanges: () => hasChanges.value,
		getOriginal: () => saved.value,
		getModified: () => changes.value,
		isSaving: () => saving.value,
		reset,
		save: saveAppearanceSettings,
	})
})

onBeforeUnmount(() => {
	theme.preview = null
	settingsModal.registerUnsavedChangesController(null)
})

provideAppearanceSettings({
	deferPersistence: true,
	theme: {
		current: computed(() => current.value.theme),
		options: themeOptions,
		system: computed(() => (theme.native === 'light' ? 'light' : preferredDarkTheme.value)),
		preferredDark: preferredDarkTheme,
		set: setTheme,
		syncAcrossDevices: {
			value: computed(() => current.value.syncAcrossDevices),
			set: setSyncAcrossDevices,
		},
		syncDisabled: computed(() => !auth.user.value),
	},
	advancedRendering: {
		value: computed(() => current.value.advancedRendering),
		set: setAdvancedRendering,
	},
	nativeDecorations:
		os !== 'macos'
			? {
					value: computed(() => current.value.nativeDecorations),
					set: setNativeDecorations,
				}
			: undefined,
	updatePreferences,
})
</script>

<template>
	<AppearanceSettingsLayout />

	<section class="mt-8 border-0 border-t border-solid border-divider pt-6">
		<div class="flex items-center justify-between gap-4">
			<div class="min-w-0">
				<h2 class="m-0 text-lg font-semibold text-contrast">
					{{ formatMessage(messages.sidebarPositionTitle) }}
				</h2>
				<p id="sidebar-position-description" class="m-0 mt-1 text-secondary">
					{{ formatMessage(messages.sidebarPositionDescription) }}
				</p>
			</div>
			<SegmentedControl
				:model-value="current.sidebarOnRight"
				:items="sidebarPositions"
				:format-label="(item: unknown) => formatSidebarPosition(Boolean(item))"
				:aria-label="formatMessage(messages.sidebarPositionTitle)"
				aria-describedby="sidebar-position-description"
				@update:model-value="
					(value) => {
						if (typeof value === 'boolean') current.sidebarOnRight = value
					}
				"
			/>
		</div>
	</section>
</template>
