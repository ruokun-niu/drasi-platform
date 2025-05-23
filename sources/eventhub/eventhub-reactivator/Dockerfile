# Copyright 2024 The Drasi Authors.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

FROM --platform=$TARGETPLATFORM mcr.microsoft.com/dotnet/aspnet@sha256:38178966094e8ac38ac3377fa4195cbf63e9aef4030f5e62d2810f8a5df769e4 AS base
# mcr.microsoft.com/dotnet/aspnet:8.0
WORKDIR /app
EXPOSE 80
EXPOSE 443

FROM --platform=$BUILDPLATFORM mcr.microsoft.com/dotnet/sdk@sha256:8c6beed050a602970c3d275756ed3c19065e42ce6ca0809f5a6fcbf5d36fd305 AS build
# mcr.microsoft.com/dotnet/sdk:8.0
WORKDIR /src
COPY ["eventhub-reactivator.csproj", "."]
RUN dotnet restore "./eventhub-reactivator.csproj"
COPY . .
WORKDIR "/src/."
RUN dotnet build "eventhub-reactivator.csproj" -c Release -o /app/build

FROM build AS publish
RUN dotnet publish "eventhub-reactivator.csproj" -c Release -o /app/publish /p:UseAppHost=false

FROM base AS final
WORKDIR /app
COPY --from=publish /app/publish .
ENTRYPOINT ["dotnet", "eventhub-reactivator.dll"]