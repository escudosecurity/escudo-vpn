package com.escudo.vpn.data.repository

import com.escudo.vpn.data.api.ApiService
import com.escudo.vpn.data.model.AuthRequest
import com.escudo.vpn.data.model.AuthResponse
import com.escudo.vpn.data.model.LoginNumberRequest
import com.escudo.vpn.data.model.ScanQrRequest
import com.escudo.vpn.data.prefs.SecurePrefs
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class AuthRepository @Inject constructor(
    private val apiService: ApiService,
    private val securePrefs: SecurePrefs
) {
    private fun saveSession(response: AuthResponse, identifier: String) {
        securePrefs.saveToken(response.token)
        securePrefs.saveUserEmail(identifier)
        securePrefs.saveUserId(response.userId)
    }

    suspend fun login(email: String, password: String): Result<AuthResponse> {
        return try {
            val response = apiService.login(AuthRequest(email, password))
            saveSession(response, email)
            Result.success(response)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun register(email: String, password: String): Result<AuthResponse> {
        return try {
            val response = apiService.register(AuthRequest(email, password))
            saveSession(response, email)
            Result.success(response)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun createAnonymousAccount(): Result<String> {
        return try {
            val response = apiService.createAnonymousAccount()
            Result.success(response.accountNumber)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun createAnonymousAccountAndLogin(): Result<String> {
        return try {
            val rawAccountNumber = apiService.createAnonymousAccount().accountNumber
            val formatted = formatAccountNumber(rawAccountNumber)
            val loginResponse = apiService.loginWithNumber(LoginNumberRequest(formatted))
            saveSession(loginResponse, formatted)
            securePrefs.saveLatestAccountNumber(formatted)
            Result.success(formatted)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun loginWithNumber(accountNumber: String): Result<AuthResponse> {
        return try {
            val formatted = formatAccountNumber(accountNumber)
            val response = apiService.loginWithNumber(LoginNumberRequest(formatted))
            saveSession(response, formatted)
            Result.success(response)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    fun formatAccountNumber(raw: String): String {
        val digits = raw.filter(Char::isDigit)
        return digits.chunked(4).joinToString("-").take(19)
    }

    fun isLoggedIn(): Boolean {
        return securePrefs.isLoggedIn()
    }

    fun getUserEmail(): String? {
        return securePrefs.getUserEmail()
    }

    fun getAccountCode(): String? {
        val latest = securePrefs.getLatestAccountNumber()
        if (!latest.isNullOrBlank()) {
            return latest
        }

        val identifier = securePrefs.getUserEmail().orEmpty()
        val digits = identifier.filter(Char::isDigit)
        return if (digits.length == 16) formatAccountNumber(identifier) else null
    }

    fun getLatestAccountNumber(): String? {
        return securePrefs.getLatestAccountNumber()
    }

    fun clearLatestAccountNumber() {
        securePrefs.clearLatestAccountNumber()
    }

    suspend fun generatePairQr(): Result<com.escudo.vpn.data.model.PairQrResponse> {
        return try {
            Result.success(apiService.generatePairQr())
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun scanQrToken(rawValue: String): Result<AuthResponse> {
        return try {
            val token = extractQrToken(rawValue)
            val response = apiService.scanQrToken(ScanQrRequest(token))
            saveSession(response, "paired-device")
            Result.success(response)
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    suspend fun getLaunchStatus(): Result<com.escudo.vpn.data.model.LaunchStatusResponse> {
        return try {
            Result.success(apiService.getLaunchStatus())
        } catch (e: Exception) {
            Result.failure(e)
        }
    }

    fun logout() {
        securePrefs.clearAll()
    }

    private fun extractQrToken(rawValue: String): String {
        val trimmed = rawValue.trim()
        val tokenFromLink = Regex("""token=([A-Za-z0-9\-]+)""").find(trimmed)?.groupValues?.getOrNull(1)
        return tokenFromLink ?: trimmed
    }
}
