package com.escudo.vpn.data.model

import com.google.gson.annotations.SerializedName

data class AuthRequest(
    val email: String,
    val password: String
)

data class LoginNumberRequest(
    @SerializedName("account_number")
    val accountNumber: String
)

data class ScanQrRequest(
    @SerializedName("qr_token")
    val qrToken: String
)

data class AuthResponse(
    val token: String,
    @SerializedName("user_id")
    val userId: String
)

data class AnonymousAccountResponse(
    @SerializedName("account_number")
    val accountNumber: String,
    val tier: String,
    @SerializedName("created_at")
    val createdAt: String
)

data class PairQrResponse(
    @SerializedName("qr_token")
    val qrToken: String,
    @SerializedName("qr_url")
    val qrUrl: String,
    @SerializedName("expires_at")
    val expiresAt: String
)

data class LaunchControls(
    @SerializedName("maintenance_mode")
    val maintenanceMode: Boolean = false,
    @SerializedName("allow_public_signup")
    val allowPublicSignup: Boolean = true,
    @SerializedName("allow_anonymous_signup")
    val allowAnonymousSignup: Boolean = true,
    @SerializedName("allow_connect")
    val allowConnect: Boolean = true,
    @SerializedName("allow_paid_checkout")
    val allowPaidCheckout: Boolean = true,
    @SerializedName("healthy_only_routing")
    val healthyOnlyRouting: Boolean = true,
    @SerializedName("expose_paid_tiers")
    val exposePaidTiers: Boolean = true,
    @SerializedName("free_beta_label")
    val freeBetaLabel: String = "Free Beta",
    @SerializedName("updated_at")
    val updatedAt: String = ""
)

data class LaunchStatusResponse(
    val controls: LaunchControls,
    @SerializedName("effective_tier")
    val effectiveTier: String,
    @SerializedName("active_invites")
    val activeInvites: Long = 0L
)

data class Server(
    val id: String,
    val name: String,
    val location: String,
    @SerializedName("country_code")
    val countryCode: String? = null,
    @SerializedName("load_percent")
    val loadPercent: Float,
    @SerializedName("service_class")
    val serviceClass: String? = null
)

data class ConnectRequest(
    @SerializedName("server_id")
    val serverId: String,
    @SerializedName("device_name")
    val deviceName: String,
    @SerializedName("device_install_id")
    val deviceInstallId: String? = null,
    val platform: String? = null,
    @SerializedName("usage_bucket")
    val usageBucket: String? = null,
    @SerializedName("preferred_class")
    val preferredClass: String? = null
)

data class ConnectResponse(
    @SerializedName("device_id")
    val deviceId: String,
    val config: String,
    @SerializedName("qr_code")
    val qrCode: String?
)

data class MultihopRequest(
    @SerializedName("entry_server_id")
    val entryServerId: String,
    @SerializedName("exit_server_id")
    val exitServerId: String,
    @SerializedName("device_name")
    val deviceName: String
)

data class PrivateModeRequest(
    @SerializedName("device_name")
    val deviceName: String
)

enum class ConnectionState {
    DISCONNECTED,
    CONNECTING,
    CONNECTED,
    DISCONNECTING
}

data class TrafficStats(
    val rxBytes: Long = 0L,
    val txBytes: Long = 0L
)

data class DnsStats(
    @SerializedName("blocked_today")
    val blockedToday: Long = 0L,
    @SerializedName("queries_today")
    val queriesToday: Long = 0L,
    @SerializedName("blocked_all_time")
    val blockedAllTime: Long = 0L
)

data class NetworkInfo(
    val ip: String,
    val country: String? = null,
    @SerializedName("country_code")
    val countryCode: String? = null,
    val city: String? = null,
    val connected: Boolean = false,
    @SerializedName("active_server_name")
    val activeServerName: String? = null,
    @SerializedName("active_server_country_code")
    val activeServerCountryCode: String? = null
)

data class FamilyOverview(
    @SerializedName("total_children")
    val totalChildren: Long = 0L,
    @SerializedName("linked_children")
    val linkedChildren: Long = 0L,
    @SerializedName("active_child_devices")
    val activeChildDevices: Long = 0L,
    @SerializedName("active_policies")
    val activePolicies: Long = 0L,
    @SerializedName("active_schedules")
    val activeSchedules: Long = 0L,
    @SerializedName("recent_events")
    val recentEvents: Long = 0L,
    val children: List<ParentalChild> = emptyList()
)

data class ParentalChild(
    val id: String,
    @SerializedName("child_user_id")
    val childUserId: String? = null,
    val name: String,
    @SerializedName("access_code")
    val accessCode: String,
    val tier: String,
    @SerializedName("is_active")
    val isActive: Boolean,
    val devices: List<ChildDevice> = emptyList(),
    val policies: List<ParentalPolicy> = emptyList(),
    val schedules: List<ParentalSchedule> = emptyList()
)

data class ChildDevice(
    val id: String,
    @SerializedName("device_id")
    val deviceId: String? = null,
    @SerializedName("device_install_id")
    val deviceInstallId: String? = null,
    @SerializedName("display_name")
    val displayName: String,
    val platform: String? = null,
    @SerializedName("is_active")
    val isActive: Boolean
)

data class ParentalPolicy(
    val id: String,
    @SerializedName("block_tiktok")
    val blockTiktok: Boolean = false,
    @SerializedName("block_youtube")
    val blockYoutube: Boolean = false,
    @SerializedName("block_social_media")
    val blockSocialMedia: Boolean = false,
    @SerializedName("block_streaming")
    val blockStreaming: Boolean = false,
    @SerializedName("bedtime_enabled")
    val bedtimeEnabled: Boolean = false,
    @SerializedName("bedtime_start_minute")
    val bedtimeStartMinute: Int? = null,
    @SerializedName("bedtime_end_minute")
    val bedtimeEndMinute: Int? = null,
    @SerializedName("max_daily_minutes")
    val maxDailyMinutes: Int? = null,
    @SerializedName("blocked_apps")
    val blockedApps: List<String> = emptyList(),
    @SerializedName("custom_blocked_domains")
    val customBlockedDomains: List<String> = emptyList()
)

data class ParentalSchedule(
    val id: String,
    val name: String,
    @SerializedName("days_of_week")
    val daysOfWeek: List<Int> = emptyList(),
    @SerializedName("start_minute")
    val startMinute: Int,
    @SerializedName("end_minute")
    val endMinute: Int,
    @SerializedName("blocked_categories")
    val blockedCategories: List<String> = emptyList(),
    @SerializedName("blocked_apps")
    val blockedApps: List<String> = emptyList(),
    @SerializedName("is_active")
    val isActive: Boolean = true
)

data class ParentalEvent(
    val id: String,
    @SerializedName("event_type")
    val eventType: String,
    @SerializedName("app_identifier")
    val appIdentifier: String? = null,
    val domain: String? = null,
    val action: String? = null,
    val detail: String? = null,
    @SerializedName("occurred_at")
    val occurredAt: String
)

data class DevicePolicyResponse(
    val child: ParentalChild? = null,
    @SerializedName("device_install_id")
    val deviceInstallId: String? = null,
    @SerializedName("device_linked")
    val deviceLinked: Boolean = false,
    @SerializedName("effective_policies")
    val effectivePolicies: List<ParentalPolicy> = emptyList(),
    @SerializedName("effective_schedules")
    val effectiveSchedules: List<ParentalSchedule> = emptyList(),
    @SerializedName("recent_events")
    val recentEvents: List<ParentalEvent> = emptyList()
)
