package com.escudo.vpn.data.repository

import com.escudo.vpn.data.api.ApiService
import com.escudo.vpn.data.model.DevicePolicyResponse
import com.escudo.vpn.data.model.FamilyOverview
import com.escudo.vpn.data.model.ParentalEvent
import com.escudo.vpn.data.prefs.SecurePrefs
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class FamilyRepository @Inject constructor(
    private val apiService: ApiService,
    private val securePrefs: SecurePrefs
) {
    fun getDeviceInstallId(): String = securePrefs.getOrCreateDeviceInstallId()

    suspend fun getFamilyOverview(): Result<FamilyOverview> {
        return try {
            Result.success(apiService.getFamilyOverview())
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun getDevicePolicy(): Result<DevicePolicyResponse> {
        return try {
            Result.success(apiService.getDevicePolicy(getDeviceInstallId()))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun getChildEvents(childId: String): Result<List<ParentalEvent>> {
        return try {
            Result.success(apiService.getChildEvents(childId))
        } catch (e: Exception) {
            Result.failure(e)
        }
    }
}
