import { ElyByColorIcon, MicrosoftColorIcon } from '@modrinth/assets'
import { type Component, computed, onScopeDispose, ref } from 'vue'

const ICON_CYCLE_INTERVAL_MS = 2500

export function useAccountIconCycle() {
	const showMicrosoft = ref(true)

	const interval = window.setInterval(() => {
		showMicrosoft.value = !showMicrosoft.value
	}, ICON_CYCLE_INTERVAL_MS)

	onScopeDispose(() => window.clearInterval(interval))

	return computed<Component>(() => (showMicrosoft.value ? MicrosoftColorIcon : ElyByColorIcon))
}
