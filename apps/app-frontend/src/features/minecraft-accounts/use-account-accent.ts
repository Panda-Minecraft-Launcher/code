import { computed, ref, watch } from 'vue'
import { useRoute } from 'vue-router'

import { get_default_user, users } from '@/helpers/auth'

type Account = {
	profile?: { id: string }
	provider?: string
}

const ELY_BY_COLOR = '#007e59'
const MICROSOFT_COLORS = ['#f25022', '#00a4ef', '#7fba00', '#ffb900']

const accounts = ref<Account[]>([])
const defaultUser = ref<string | undefined>()
const microsoftIndex = ref(0)

const selectedAccount = computed(() =>
	accounts.value.find((account) => account.profile?.id === defaultUser.value),
)
const hasAccount = computed(() => !!selectedAccount.value)
const provider = computed(() => {
	const selected = selectedAccount.value
	if (!selected) return null
	return selected.provider?.toLowerCase() === 'elyby' ? 'elyby' : 'microsoft'
})

function withAlpha(color: string, alpha: number) {
	const value = Number.parseInt(color.slice(1), 16)
	return `rgba(${(value >> 16) & 255}, ${(value >> 8) & 255}, ${value & 255}, ${alpha})`
}

export async function refreshAccountAccent() {
	defaultUser.value = await get_default_user().catch(() => undefined)
	accounts.value = ((await users().catch(() => [])) as Account[]) ?? []
}

export function useAccountAccent() {
	const route = useRoute()

	watch(
		() => route.path,
		() => {
			if (provider.value === 'microsoft') microsoftIndex.value++
		},
	)

	watch(
		[provider, microsoftIndex],
		() => {
			const color =
				provider.value === 'elyby'
					? ELY_BY_COLOR
					: provider.value === 'microsoft'
						? MICROSOFT_COLORS[microsoftIndex.value % MICROSOFT_COLORS.length]
						: 'var(--color-gray)'
			const style = document.documentElement.style
			style.setProperty('--color-brand', color)
			style.setProperty(
				'--color-brand-highlight',
				provider.value ? withAlpha(color, 0.25) : 'var(--color-gray-highlight)',
			)
			style.setProperty(
				'--color-brand-shadow',
				provider.value ? withAlpha(color, 0.7) : 'var(--color-gray)',
			)
		},
		{ immediate: true },
	)

	void refreshAccountAccent()

	return { hasAccount }
}
