inspect_asm::alloc_u32::try_bumpalo:
	push rbx
	mov rcx, qword ptr [rdi + 16]
	mov rax, qword ptr [rcx + 32]
	cmp rax, 4
	jb .LBB_3
	add rax, -4
	and rax, -4
	cmp rax, qword ptr [rcx]
	jb .LBB_3
	mov qword ptr [rcx + 32], rax
	test rax, rax
	je .LBB_3
.LBB_5:
	mov dword ptr [rax], esi
	pop rbx
	ret
.LBB_3:
	mov ebx, esi
	mov esi, 4
	mov edx, 4
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov esi, ebx
	test rax, rax
	jne .LBB_5
	xor eax, eax
	pop rbx
	ret