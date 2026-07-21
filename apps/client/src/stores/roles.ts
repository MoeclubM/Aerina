import { defineStore } from "pinia";
import { computed, ref, watch } from "vue";

export interface RoleAssistant {
  id: string;
  name: string;
  icon: string;
  description: string;
  systemPrompt: string;
  temperature: number;
  modelPresetId?: string | null;
  isBuiltin?: boolean;
}

const DEFAULT_ROLE_ID_KEY = "aerina.defaultRoleId";
const CUSTOM_ROLES_KEY = "aerina.customRoles";
const CONV_ROLES_KEY = "aerina.convRoles";

export const BUILTIN_ROLES: RoleAssistant[] = [
  {
    id: "default",
    name: "默认助手",
    icon: "mdi-robot-outline",
    description: "通用示例角色，适合日常问答与综合任务。",
    systemPrompt: "You are a helpful, clear, and thoughtful AI assistant.",
    temperature: 0.7,
    isBuiltin: true,
  },
  {
    id: "coder",
    name: "编程助手",
    icon: "mdi-code-tags",
    description: "展示专业角色效果，偏重代码质量、分析与解释。",
    systemPrompt: "You are an expert software engineer. Provide clean, efficient, and well-structured code with clear explanations.",
    temperature: 0.5,
    isBuiltin: true,
  },
];

function readCustomRoles(): RoleAssistant[] {
  try {
    const raw = localStorage.getItem(CUSTOM_ROLES_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    if (Array.isArray(parsed)) return parsed;
  } catch (error) {
    console.error("Failed to parse custom roles", error);
  }
  return [];
}

function readConvRoles(): Record<string, string> {
  try {
    const raw = localStorage.getItem(CONV_ROLES_KEY);
    if (raw) return JSON.parse(raw);
  } catch (error) {
    console.error("Failed to parse conversation roles", error);
  }
  return {};
}

export const useRoleStore = defineStore("roles", () => {
  const customRoles = ref<RoleAssistant[]>(readCustomRoles());
  const roles = computed(() => [...BUILTIN_ROLES, ...customRoles.value]);
  const savedDefaultRoleId = localStorage.getItem(DEFAULT_ROLE_ID_KEY) || "default";
  const defaultRoleId = ref(
    roles.value.some((role) => role.id === savedDefaultRoleId) ? savedDefaultRoleId : "default",
  );
  const convRoles = ref<Record<string, string>>(readConvRoles());

  watch(
    customRoles,
    (value) => {
      localStorage.setItem(CUSTOM_ROLES_KEY, JSON.stringify(value));
    },
    { deep: true },
  );

  watch(defaultRoleId, (value) => {
    localStorage.setItem(DEFAULT_ROLE_ID_KEY, value);
  });

  watch(
    convRoles,
    (value) => {
      localStorage.setItem(CONV_ROLES_KEY, JSON.stringify(value));
    },
    { deep: true },
  );

  const defaultRole = computed(() =>
    roles.value.find((role) => role.id === defaultRoleId.value) || BUILTIN_ROLES[0],
  );

  function setDefaultRole(id: string) {
    if (roles.value.some((role) => role.id === id)) defaultRoleId.value = id;
  }

  function addRole(newRole: Omit<RoleAssistant, "id" | "isBuiltin">) {
    const role: RoleAssistant = {
      ...newRole,
      id: `role-${crypto.randomUUID()}`,
      isBuiltin: false,
    };
    customRoles.value.push(role);
    return role;
  }

  function updateRole(id: string, updated: Omit<RoleAssistant, "id" | "isBuiltin">) {
    const index = customRoles.value.findIndex((role) => role.id === id);
    if (index !== -1) customRoles.value[index] = { ...customRoles.value[index], ...updated };
  }

  function deleteRole(id: string) {
    const index = customRoles.value.findIndex((role) => role.id === id);
    if (index === -1) return;
    customRoles.value.splice(index, 1);
    if (defaultRoleId.value === id) defaultRoleId.value = "default";
  }

  function setConversationRole(convId: string, roleId: string) {
    convRoles.value[convId] = roleId;
  }

  function getConversationRole(convId: string): RoleAssistant {
    const roleId = convRoles.value[convId] || defaultRoleId.value;
    return roles.value.find((role) => role.id === roleId) || defaultRole.value;
  }

  return {
    roles,
    defaultRoleId,
    defaultRole,
    convRoles,
    setDefaultRole,
    addRole,
    updateRole,
    deleteRole,
    setConversationRole,
    getConversationRole,
  };
});
