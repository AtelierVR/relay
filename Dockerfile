# Stage 1: Build the application
FROM mcr.microsoft.com/dotnet/sdk:8.0-alpine AS build
WORKDIR /app
COPY ./src/ ./src/
COPY ./NoxRelay.csproj ./
COPY ./Program.cs ./
RUN dotnet restore ./NoxRelay.csproj
RUN dotnet publish ./NoxRelay.csproj -c Release -o out

# Stage 2: Build the runtime image
FROM mcr.microsoft.com/dotnet/aspnet:8.0-alpine AS runtime
WORKDIR /app
COPY --from=build /app/out ./
ENTRYPOINT ["dotnet", "NoxRelay.dll"]