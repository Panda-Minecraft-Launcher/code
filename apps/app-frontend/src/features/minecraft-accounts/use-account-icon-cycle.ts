import { ElyByColorIcon, MicrosoftColorIcon } from '@modrinth/assets'
import { type Component, computed, onScopeDispose, ref } from 'vue'

const ICON_CYCLE_INTERVAL_MS = 2500

const showMicrosoft = ref(true)
let subscribers = 0
let interval: number | undefined

export function useAccountIconCycle() {
	subscribers++
	if (interval === undefined) {
		interval = window.setInterval(() => {
			showMicrosoft.value = !showMicrosoft.value
		}, ICON_CYCLE_INTERVAL_MS)
	}

	onScopeDispose(() => {
		if (--subscribers === 0) {
			window.clearInterval(interval)
			interval = undefined
		}
	})

	return computed<Component>(() => (showMicrosoft.value ? MicrosoftColorIcon : ElyByColorIcon))
}
