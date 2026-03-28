package com.escudo.vpn.data.repository

import android.os.Build
import com.escudo.vpn.data.api.ApiService
import com.escudo.vpn.data.model.ConnectRequest
import com.escudo.vpn.data.model.ConnectResponse
import com.escudo.vpn.data.model.DnsStats
import com.escudo.vpn.data.model.MultihopRequest
import com.escudo.vpn.data.model.NetworkInfo
import com.escudo.vpn.data.model.Server
import com.escudo.vpn.data.model.PrivateModeRequest
import com.escudo.vpn.data.prefs.SecurePrefs
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class VpnRepository @Inject constructor(
    private val apiService: ApiService,
    private val securePrefs: SecurePrefs
) {
    private fun connectRequest(serverId: String, deviceName: String): ConnectRequest {
        return ConnectRequest(
            serverId = serverId,
            deviceName = deviceName,
            deviceInstallId = securePrefs.getOrCreateDeviceInstallId(),
            platform = "android",
            usageBucket = "normal",
            preferredClass = "free"
        )
    }

    suspend fun getServers(): Result<List<Server>> {
        return try {
            val servers = apiService.getServers()
            Result.success(servers)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun connect(serverId: String): Result<ConnectResponse> {
        return try {
            val deviceName = "${Build.MANUFACTURER} ${Build.MODEL}"
            val response = apiService.connect(connectRequest(serverId, deviceName))
            securePrefs.saveDeviceId(response.deviceId)
            Result.success(response)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun disconnect(): Result<Unit> {
        return try {
            val deviceId = securePrefs.getDeviceId()
            if (deviceId != null) {
                val response = apiService.disconnect(deviceId)
                if (!response.isSuccessful) {
                    return Result.failure(
                        IllegalStateException("Disconnect failed with HTTP ${response.code()}")
                    )
                }
                securePrefs.clearDeviceId()
            }
            Result.success(Unit)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun connectMultihop(entryServerId: String, exitServerId: String): Result<ConnectResponse> {
        return try {
            val deviceName = "${Build.MANUFACTURER} ${Build.MODEL}"
            val response = apiService.connectMultihop(
                MultihopRequest(entryServerId, exitServerId, deviceName)
            )
            securePrefs.saveDeviceId(response.deviceId)
            Result.success(response)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun connectPrivateMode(): Result<ConnectResponse> {
        return try {
            val deviceName = "${Build.MANUFACTURER} ${Build.MODEL}"
            val response = apiService.connectPrivateMode(
                PrivateModeRequest(deviceName)
            )
            securePrefs.saveDeviceId(response.deviceId)
            Result.success(response)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    fun saveSelectedServer(id: String, name: String) {
        securePrefs.saveSelectedServer(id, name)
    }

    fun getSelectedServerId(): String? {
        return securePrefs.getSelectedServerId()
    }

    fun getSelectedServerName(): String? {
        return securePrefs.getSelectedServerName()
    }

    fun isKillSwitchEnabled(): Boolean {
        return securePrefs.isKillSwitchEnabled()
    }

    fun setKillSwitch(enabled: Boolean) {
        securePrefs.setKillSwitch(enabled)
    }

    suspend fun getDnsStats(): Result<DnsStats> {
        return try {
            val stats = apiService.getDnsStats()
            Result.success(stats)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun getNetworkInfo(): Result<NetworkInfo> {
        return try {
            val info = apiService.getNetworkInfo()
            Result.success(info)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}
