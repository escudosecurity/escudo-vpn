package com.escudo.vpn.data.api

import com.escudo.vpn.data.model.AuthRequest
import com.escudo.vpn.data.model.AuthResponse
import com.escudo.vpn.data.model.AnonymousAccountResponse
import com.escudo.vpn.data.model.ConnectRequest
import com.escudo.vpn.data.model.ConnectResponse
import com.escudo.vpn.data.model.DevicePolicyResponse
import com.escudo.vpn.data.model.DnsStats
import com.escudo.vpn.data.model.FamilyOverview
import com.escudo.vpn.data.model.LaunchStatusResponse
import com.escudo.vpn.data.model.LoginNumberRequest
import com.escudo.vpn.data.model.MultihopRequest
import com.escudo.vpn.data.model.NetworkInfo
import com.escudo.vpn.data.model.ParentalEvent
import com.escudo.vpn.data.model.PairQrResponse
import com.escudo.vpn.data.model.Server
import com.escudo.vpn.data.model.PrivateModeRequest
import com.escudo.vpn.data.model.ScanQrRequest
import retrofit2.http.Body
import retrofit2.http.DELETE
import retrofit2.http.GET
import retrofit2.http.POST
import retrofit2.http.Path
import retrofit2.http.Query

interface ApiService {

    @POST("api/v1/auth/register")
    suspend fun register(@Body request: AuthRequest): AuthResponse

    @POST("api/v1/auth/login")
    suspend fun login(@Body request: AuthRequest): AuthResponse

    @POST("api/v1/auth/anonymous")
    suspend fun createAnonymousAccount(): AnonymousAccountResponse

    @POST("api/v1/auth/login-number")
    suspend fun loginWithNumber(@Body request: LoginNumberRequest): AuthResponse

    @POST("api/v1/auth/qr/generate")
    suspend fun generatePairQr(): PairQrResponse

    @POST("api/v1/auth/qr/scan")
    suspend fun scanQrToken(@Body request: ScanQrRequest): AuthResponse

    @GET("api/v1/launch/status")
    suspend fun getLaunchStatus(): LaunchStatusResponse

    @GET("api/v1/servers")
    suspend fun getServers(): List<Server>

    @POST("api/v1/connect")
    suspend fun connect(@Body request: ConnectRequest): ConnectResponse

    @DELETE("api/v1/disconnect/{id}")
    suspend fun disconnect(@Path("id") deviceId: String): retrofit2.Response<Unit>

    @POST("api/v1/connect/multihop")
    suspend fun connectMultihop(@Body request: MultihopRequest): ConnectResponse

    @POST("api/v1/connect/private-mode")
    suspend fun connectPrivateMode(@Body request: PrivateModeRequest): ConnectResponse

    @GET("api/v1/stats/dns")
    suspend fun getDnsStats(): DnsStats

    @GET("api/v1/network/me")
    suspend fun getNetworkInfo(): NetworkInfo

    @GET("api/v1/family/parental/overview")
    suspend fun getFamilyOverview(): FamilyOverview

    @GET("api/v1/family/parental/device-policy")
    suspend fun getDevicePolicy(@Query("device_install_id") deviceInstallId: String): DevicePolicyResponse

    @GET("api/v1/family/parental/children/{id}/events")
    suspend fun getChildEvents(@Path("id") childId: String): List<ParentalEvent>
}
