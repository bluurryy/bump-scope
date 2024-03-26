inspect_asm::alloc_vec3::try_up:
	push rbx
	mov rcx, qword ptr [rdi]
	mov rax, qword ptr [rcx]
	mov rdx, qword ptr [rcx + 8]
	add rax, 3
	and rax, -4
	sub rdx, rax
	cmp rdx, 12
	jb .LBB_3
	lea rdx, [rax + 12]
	mov qword ptr [rcx], rdx
.LBB_2:
	mov ecx, dword ptr [rsi + 8]
	mov dword ptr [rax + 8], ecx
	mov rcx, qword ptr [rsi]
	mov qword ptr [rax], rcx
	pop rbx
	ret
.LBB_3:
	mov rbx, rsi
	call bump_scope::bump_scope::BumpScope<_,_,A>::do_alloc_sized_in_another_chunk
	mov rsi, rbx
	test rax, rax
	jne .LBB_2
	xor eax, eax
	pop rbx
	ret