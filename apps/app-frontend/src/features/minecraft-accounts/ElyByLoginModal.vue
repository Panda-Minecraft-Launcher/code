<template>
	<NewModal ref="modal" :header="formatMessage(messages.header)" max-width="480px">
		<div class="flex flex-col gap-5 p-6">
			<p class="m-0 text-sm text-secondary">
				{{ formatMessage(messages.instructions) }}
			</p>

			<div class="flex flex-col gap-2">
				<span class="text-xs font-semibold uppercase text-secondary">
					{{ formatMessage(messages.codeLabel) }}
				</span>
				<div class="flex items-center gap-2">
					<code
						class="flex-grow text-center text-2xl font-mono tracking-widest bg-surface-2 border border-solid border-surface-5 rounded-lg py-3 select-all"
					>
						{{ userCode }}
					</code>
					<Button type="outlined" :disabled="copied" @click="copyCode">
						<CheckIcon v-if="copied" />
						<CopyIcon v-else />
						{{ copied ? formatMessage(messages.copied) : formatMessage(messages.copy) }}
					</Button>
				</div>
			</div>

			<Button type="colored" color="brand" class="w-full" @click="openVerificationPage">
				<ExternalIcon />
				{{ formatMessage(messages.openPage) }}
			</Button>

			<div v-if="waiting" class="flex items-center gap-2 text-sm text-secondary">
				<SpinnerIcon class="animate-spin w-4 h-4 shrink-0" />
				{{ formatMessage(messages.waiting) }}
			</div>
		</div>
	</NewModal>
</template>

<script setup lang="ts">
import { CheckIcon, CopyIcon, ExternalIcon, SpinnerIcon } from '@modrinth/assets'
import { Button, defineMessages, NewModal, useVIntl } from '@modrinth/ui'
import { onUnmounted, ref } from 'vue'

import { handleSevereError } from '@/composables/use-error.js'
import { injectMinecraftAccounts } from '@/features/minecraft-accounts/context'
import { login_elyby_begin, login_elyby_finish, set_default_user } from '@/helpers/auth'
import { openExternalUrl } from '@/platform/external-links'

const emit = defineEmits<{
	success: []
}>()

const { formatMessage } = useVIntl()
const minecraftAccounts = injectMinecraftAccounts()

const messages = defineMessages({
	header: {
		id: 'minecraft-account.elyby.header',
		defaultMessage: 'Sign in with ely.by',
	},
	instructions: {
		id: 'minecraft-account.elyby.instructions',
		defaultMessage:
			'Open the ely.by verification page and enter this code to authorize Panda Minecraft Launcher. Sign-in will continue automatically once you approve it.',
	},
	codeLabel: {
		id: 'minecraft-account.elyby.code-label',
		defaultMessage: 'Your code',
	},
	copy: {
		id: 'minecraft-account.elyby.copy',
		defaultMessage: 'Copy',
	},
	copied: {
		id: 'minecraft-account.elyby.copied',
		defaultMessage: 'Copied',
	},
	openPage: {
		id: 'minecraft-account.elyby.open-page',
		defaultMessage: 'Open ely.by verification page',
	},
	waiting: {
		id: 'minecraft-account.elyby.waiting',
		defaultMessage: 'Waiting for authorization…',
	},
})

const modal = ref<InstanceType<typeof NewModal>>()
const userCode = ref('')
const verificationUrl = ref('')
const deviceCode = ref('')
const waiting = ref(false)
const copied = ref(false)
let copiedTimeout: ReturnType<typeof setTimeout> | undefined

onUnmounted(() => {
	if (copiedTimeout) clearTimeout(copiedTimeout)
})

function show() {
	modal.value?.show()
}

function hide() {
	modal.value?.hide()
}

async function signIn() {
	waiting.value = false
	copied.value = false

	try {
		const device = await login_elyby_begin()
		deviceCode.value = device.device_code
		userCode.value = device.user_code
		verificationUrl.value = device.verification_uri_complete || device.verification_uri
		show()

		waiting.value = true
		const loggedIn = await login_elyby_finish(deviceCode.value)

		await set_default_user(loggedIn.profile.id)
		await minecraftAccounts.value?.refreshValues()

		hide()
		emit('success')
	} catch (error) {
		hide()
		throw error
	} finally {
		waiting.value = false
	}
}

async function copyCode() {
	try {
		await navigator.clipboard.writeText(userCode.value)
		copied.value = true
		if (copiedTimeout) clearTimeout(copiedTimeout)
		copiedTimeout = setTimeout(() => {
			copied.value = false
		}, 3000)
	} catch (error) {
		handleSevereError(error)
	}
}

async function openVerificationPage() {
	await openExternalUrl(verificationUrl.value).catch(handleSevereError)
}

defineExpose({
	signIn,
})
</script>
