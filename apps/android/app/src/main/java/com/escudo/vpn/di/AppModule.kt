package com.escudo.vpn.di

import android.content.Context
import com.escudo.vpn.BuildConfig
import com.escudo.vpn.data.api.ApiClientFactory
import com.escudo.vpn.data.api.ApiService
import com.escudo.vpn.data.api.AuthInterceptor
import com.escudo.vpn.data.prefs.SecurePrefs
import com.escudo.vpn.service.TunnelManager
import dagger.Module
import dagger.Provides
import dagger.hilt.InstallIn
import dagger.hilt.android.qualifiers.ApplicationContext
import dagger.hilt.components.SingletonComponent
import okhttp3.OkHttpClient
import retrofit2.Retrofit
import javax.inject.Singleton

@Module
@InstallIn(SingletonComponent::class)
object AppModule {

    @Provides
    @Singleton
    fun provideSecurePrefs(@ApplicationContext context: Context): SecurePrefs {
        return SecurePrefs(context)
    }

    @Provides
    @Singleton
    fun provideAuthInterceptor(securePrefs: SecurePrefs): AuthInterceptor {
        return AuthInterceptor(securePrefs)
    }

    @Provides
    @Singleton
    fun provideOkHttpClient(authInterceptor: AuthInterceptor): OkHttpClient {
        return ApiClientFactory.createOkHttpClient(authInterceptor)
    }

    @Provides
    @Singleton
    fun provideRetrofit(okHttpClient: OkHttpClient): Retrofit {
        return ApiClientFactory.createRetrofit(okHttpClient)
    }

    @Provides
    @Singleton
    fun provideApiService(retrofit: Retrofit): ApiService {
        return retrofit.create(ApiService::class.java)
    }

    @Provides
    @Singleton
    fun provideTunnelManager(): TunnelManager {
        return TunnelManager()
    }
}
