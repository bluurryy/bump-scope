inspect_asm::alloc_u32::try_bumpalo:
	push rbx
	mov rcx, qword ptr [rdi + 16]
	mov rax, qword ptr [rcx + 32]
	cmp rax, 4
	jb .LBB_4
	add rax, -4
	and rax, -4
	cmp rax, qword ptr [rcx]
	jb .LBB_4
	mov qword ptr [rcx + 32], rax
.LBB_3:
	mov dword ptr [rax], esi
	pop rbx
	ret
.LBB_4:
	mov ebx, esi
	mov esi, 4
	mov edx, 4
	call qword ptr [rip + bumpalo::Bump::alloc_layout_slow@GOTPCREL]
	mov esi, ebx
	test rax, rax
	jne .LBB_3
	xor eax, eax
	pop rbx
	ret