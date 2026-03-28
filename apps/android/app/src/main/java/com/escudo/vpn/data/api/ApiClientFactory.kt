package com.escudo.vpn.data.api

import com.escudo.vpn.BuildConfig
import okhttp3.CertificatePinner
import okhttp3.OkHttpClient
import okhttp3.logging.HttpLoggingInterceptor
import retrofit2.Retrofit
import retrofit2.converter.gson.GsonConverterFactory
import java.util.concurrent.TimeUnit

object ApiClientFactory {
    fun createOkHttpClient(authInterceptor: AuthInterceptor): OkHttpClient {
        val loggingInterceptor = HttpLoggingInterceptor().apply {
            level = if (BuildConfig.DEBUG) {
                HttpLoggingInterceptor.Level.BODY
            } else {
                HttpLoggingInterceptor.Level.NONE
            }
        }

        val builder = OkHttpClient.Builder()
            .addInterceptor(authInterceptor)
            .addInterceptor(loggingInterceptor)
            .connectTimeout(30, TimeUnit.SECONDS)
            .readTimeout(30, TimeUnit.SECONDS)
            .writeTimeout(30, TimeUnit.SECONDS)

        val configuredPins = BuildConfig.CERT_PINS
            .split(",")
            .map { it.trim() }
            .filter { it.isNotEmpty() }
        if (!BuildConfig.DEBUG && BuildConfig.PINNED_DOMAIN.isNotBlank() && configuredPins.isNotEmpty()) {
            val certificatePinner = CertificatePinner.Builder().apply {
                configuredPins.forEach { pin ->
                    add(BuildConfig.PINNED_DOMAIN, pin)
                }
            }.build()
            builder.certificatePinner(certificatePinner)
        }

        return builder.build()
    }

    fun createRetrofit(okHttpClient: OkHttpClient): Retrofit {
        return Retrofit.Builder()
            .baseUrl(BuildConfig.API_BASE_URL)
            .client(okHttpClient)
            .addConverterFactory(GsonConverterFactory.create())
            .build()
    }
}
