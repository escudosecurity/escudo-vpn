package com.escudo.vpn.data.prefs

import android.content.Context
import android.content.SharedPreferences
import androidx.security.crypto.EncryptedSharedPreferences
import androidx.security.crypto.MasterKey
import com.escudo.vpn.BuildConfig
import java.util.UUID
import dagger.hilt.android.qualifiers.ApplicationContext
import javax.inject.Inject
import javax.inject.Singleton

@Singleton
class SecurePrefs @Inject constructor(
    @ApplicationContext private val context: Context
) {
    companion object {
        private const val PREFS_NAME = "escudo_secure_prefs"
        private const val KEY_TOKEN = "auth_token"
        private const val KEY_USER_EMAIL = "user_email"
        private const val KEY_USER_ID = "user_id"
        private const val KEY_KILL_SWITCH = "kill_switch"
        private const val KEY_DEVICE_ID = "device_id"
        private const val KEY_SELECTED_SERVER_ID = "selected_server_id"
        private const val KEY_SELECTED_SERVER_NAME = "selected_server_name"
        private const val KEY_AUTO_PROTECT = "auto_protect"
        private const val KEY_TRUSTED_NETWORKS = "trusted_networks"
        private const val KEY_DEVICE_INSTALL_ID = "device_install_id"
        private const val KEY_LATEST_ACCOUNT_NUMBER = "latest_account_number"
    }

    private val prefs: SharedPreferences? by lazy {
        try {
            val masterKey = MasterKey.Builder(context)
                .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                .build()

            EncryptedSharedPreferences.create(
                context,
                PREFS_NAME,
                masterKey,
                EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
            )
        } catch (e: Exception) {
            // Fallback: delete corrupted prefs and recreate
            if (BuildConfig.DEBUG) {
                android.util.Log.e("SecurePrefs", "Encrypted prefs corrupted, recreating", e)
            } else {
                android.util.Log.e("SecurePrefs", "Encrypted prefs corrupted, recreating")
            }
            context.getSharedPreferences("escudo_prefs_fallback", Context.MODE_PRIVATE).edit().clear().apply()
            try {
                val prefsFile = java.io.File(context.filesDir.parent, "shared_prefs/escudo_secure_prefs.xml")
                if (prefsFile.exists()) prefsFile.delete()
                val masterKeyFile = java.io.File(context.filesDir.parent, "shared_prefs/_androidx_security_master_key_.xml")
                if (masterKeyFile.exists()) masterKeyFile.delete()

                val masterKey = MasterKey.Builder(context)
                    .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                    .build()

                EncryptedSharedPreferences.create(
                    context,
                    PREFS_NAME,
                    masterKey,
                    EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
                    EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
                )
            } catch (e2: Exception) {
                if (BuildConfig.DEBUG) {
                    android.util.Log.e("SecurePrefs", "Failed to recreate encrypted prefs; secure storage unavailable", e2)
                } else {
                    android.util.Log.e("SecurePrefs", "Secure storage unavailable")
                }
                null
            }
        }
    }

    fun saveToken(token: String) {
        prefs?.edit()?.putString(KEY_TOKEN, token)?.apply()
    }

    fun getToken(): String? {
        return prefs?.getString(KEY_TOKEN, null)
    }

    fun saveUserEmail(email: String) {
        prefs?.edit()?.putString(KEY_USER_EMAIL, email)?.apply()
    }

    fun getUserEmail(): String? {
        return prefs?.getString(KEY_USER_EMAIL, null)
    }

    fun saveUserId(id: String) {
        prefs?.edit()?.putString(KEY_USER_ID, id)?.apply()
    }

    fun getUserId(): String? {
        return prefs?.getString(KEY_USER_ID, null)
    }

    fun saveLatestAccountNumber(accountNumber: String) {
        prefs?.edit()?.putString(KEY_LATEST_ACCOUNT_NUMBER, accountNumber)?.apply()
    }

    fun getLatestAccountNumber(): String? {
        return prefs?.getString(KEY_LATEST_ACCOUNT_NUMBER, null)
    }

    fun clearLatestAccountNumber() {
        prefs?.edit()?.remove(KEY_LATEST_ACCOUNT_NUMBER)?.apply()
    }

    fun setKillSwitch(enabled: Boolean) {
        prefs?.edit()?.putBoolean(KEY_KILL_SWITCH, enabled)?.apply()
    }

    fun isKillSwitchEnabled(): Boolean {
        return prefs?.getBoolean(KEY_KILL_SWITCH, false) ?: false
    }

    fun saveDeviceId(deviceId: String) {
        prefs?.edit()?.putString(KEY_DEVICE_ID, deviceId)?.apply()
    }

    fun getDeviceId(): String? {
        return prefs?.getString(KEY_DEVICE_ID, null)
    }

    fun clearDeviceId() {
        prefs?.edit()?.remove(KEY_DEVICE_ID)?.apply()
    }

    fun getOrCreateDeviceInstallId(): String {
        val existing = prefs?.getString(KEY_DEVICE_INSTALL_ID, null)
        if (!existing.isNullOrBlank()) {
            return existing
        }

        val generated = UUID.randomUUID().toString()
        prefs?.edit()?.putString(KEY_DEVICE_INSTALL_ID, generated)?.apply()
        return generated
    }

    fun saveSelectedServer(id: String, name: String) {
        prefs?.edit()?.apply {
            putString(KEY_SELECTED_SERVER_ID, id)
            putString(KEY_SELECTED_SERVER_NAME, name)
            apply()
        }
    }

    fun getSelectedServerId(): String? {
        return prefs?.getString(KEY_SELECTED_SERVER_ID, null)
    }

    fun getSelectedServerName(): String? {
        return prefs?.getString(KEY_SELECTED_SERVER_NAME, null)
    }

    fun clearAll() {
        prefs?.edit()?.clear()?.apply()
    }

    fun isAutoProtectEnabled(): Boolean {
        return prefs?.getBoolean(KEY_AUTO_PROTECT, true) ?: true
    }

    fun setAutoProtectEnabled(enabled: Boolean) {
        prefs?.edit()?.putBoolean(KEY_AUTO_PROTECT, enabled)?.apply()
    }

    fun getTrustedNetworks(): Set<String> {
        return prefs?.getStringSet(KEY_TRUSTED_NETWORKS, emptySet()) ?: emptySet()
    }

    fun addTrustedNetwork(ssid: String) {
        val networks = getTrustedNetworks().toMutableSet()
        networks.add(ssid)
        prefs?.edit()?.putStringSet(KEY_TRUSTED_NETWORKS, networks)?.apply()
    }

    fun removeTrustedNetwork(ssid: String) {
        val networks = getTrustedNetworks().toMutableSet()
        networks.remove(ssid)
        prefs?.edit()?.putStringSet(KEY_TRUSTED_NETWORKS, networks)?.apply()
    }

    fun isLoggedIn(): Boolean {
        return !getToken().isNullOrEmpty()
    }
}
